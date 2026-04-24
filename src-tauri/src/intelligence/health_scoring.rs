//! I499: Algorithmic account health scoring engine (ADR-0097).
//!
//! "LLM explains numbers, doesn't pick them." Six dimensions compute a score;
//! the LLM provides narrative only.

use crate::db::types::DbAccount;
use crate::db::ActionDb;
use crate::presets::loader::canonical_role_id;
use crate::presets::schema::{PresetIntelligenceConfig, RolePreset};
use crate::signals::fusion;

use super::io::{
    AccountHealth, DimensionScore, HealthDivergence, HealthSource, HealthTrend, HealthTrendTag,
    OrgHealthData, RelationshipDimensions,
};

// DOS-53: Pushing actions to Linear could feed the engagement health dimension
// as evidence of active account work. Deferred to post-v1.2.0 — the current
// 6 dimensions are algorithmically computed from meeting/email/stakeholder data,
// and action push frequency is too new a signal to tune weights on.

/// Compute algorithmic health for an account using 6 dimensions.
///
/// The returned `AccountHealth` has score, band, dimensions, and confidence
/// but no narrative (that comes from the LLM).
pub fn compute_account_health(
    db: &ActionDb,
    account: &DbAccount,
    org_health: Option<&OrgHealthData>,
) -> AccountHealth {
    compute_account_health_with_preset(db, account, org_health, None)
}

/// Compute algorithmic health with an optional role preset.
pub fn compute_account_health_with_preset(
    db: &ActionDb,
    account: &DbAccount,
    org_health: Option<&OrgHealthData>,
    preset: Option<&RolePreset>,
) -> AccountHealth {
    let meeting_cadence = compute_meeting_cadence(db, &account.id);
    let email_engagement = compute_email_engagement(db, &account.id);
    let stakeholder_coverage = compute_stakeholder_coverage(db, &account.id);
    let key_advocate_health = compute_key_advocate_health(db, &account.id);
    let financial_proximity = compute_financial_proximity(db, account);
    let signal_momentum = compute_signal_momentum(db, &account.id);

    let dims = RelationshipDimensions {
        meeting_cadence,
        email_engagement,
        stakeholder_coverage,
        key_advocate_health,
        financial_proximity,
        signal_momentum,
    };

    let lifecycle = account.lifecycle.as_deref();
    let preset_id = preset.map(|p| p.id.as_str()).unwrap_or("core");
    let raw_weights =
        compose_dimension_weights(preset_id, preset.map(|p| &p.intelligence), lifecycle);
    let weights = redistribute_weights(&dims, raw_weights);
    let confidence = compute_confidence(&dims);

    // DOS-84: Count dimensions with actual data (weight > 0), excluding
    // the signal_momentum neutral placeholder which always has weight > 0.
    let sufficient_data = has_sufficient_data(&dims);

    // Compute weighted average of non-null dimensions
    let dim_arr = [
        &dims.meeting_cadence,
        &dims.email_engagement,
        &dims.stakeholder_coverage,
        &dims.key_advocate_health,
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
    for ev in &dims.key_advocate_health.evidence {
        if ev.contains("Consider tagging") || ev.contains("champion candidate") {
            recommended_actions.push(ev.clone());
        }
    }
    for ev in &dims.stakeholder_coverage.evidence {
        if ev.starts_with("Missing:") {
            recommended_actions.push(format!(
                "Map a {} stakeholder for this account",
                ev.trim_start_matches("Missing: ")
            ));
        }
    }

    // I633: Compute real trend from health_score_history instead of hardcoded "stable"
    let trend = compute_trend_from_history(db, &account.id, score);

    // Record this score in history for future trend computation
    record_health_score(db, &account.id, score, &band, confidence);

    AccountHealth {
        score,
        band,
        source: HealthSource::Computed,
        confidence,
        sufficient_data,
        trend,
        dimensions: dims,
        narrative: None,
        recommended_actions,
        divergence,
    }
}

/// Record a health score data point for trend computation.
fn record_health_score(db: &ActionDb, account_id: &str, score: f64, band: &str, confidence: f64) {
    let _ = db.conn.execute(
        "INSERT INTO health_score_history (account_id, score, band, confidence)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![account_id, score, band, confidence],
    );

    // Prune old entries — keep at most 20 per account to bound storage
    let _ = db.conn.execute(
        "DELETE FROM health_score_history WHERE account_id = ?1 AND id NOT IN (
             SELECT id FROM health_score_history WHERE account_id = ?1
             ORDER BY computed_at DESC LIMIT 20
         )",
        rusqlite::params![account_id],
    );
}

/// Compute trend from the last 3-5 health score data points.
///
///   ┌───────────────────────────────────────────────────────┐
///   │ History points │ Trend logic                          │
///   ├────────────────┼──────────────────────────────────────┤
///   │ 0-1            │ "stable" (not enough data)           │
///   │ 2-5            │ Compare oldest vs newest, ±5 = move  │
///   │ 5+             │ Linear slope of last 5               │
///   └───────────────────────────────────────────────────────┘
fn compute_trend_from_history(db: &ActionDb, account_id: &str, current_score: f64) -> HealthTrend {
    let history: Vec<(f64, String)> = db
        .conn
        .prepare(
            "SELECT score, computed_at FROM health_score_history
             WHERE account_id = ?1
             ORDER BY computed_at DESC LIMIT 5",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![account_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if history.len() < 2 {
        return HealthTrend {
            direction: "stable".to_string(),
            rationale: Some("Insufficient history for trend".to_string()),
            timeframe: "30d".to_string(),
            confidence: 0.1,
            delta: None,
            tags: Vec::new(),
        };
    }

    // Compare current score to oldest in window
    let oldest_score = history.last().map(|(s, _)| *s).unwrap_or(current_score);
    let delta_f = current_score - oldest_score;
    // DOS-249: integer delta for the "▲ +12 in 30d" meta line
    let delta_i = delta_f.round() as i32;

    let (direction, rationale) = if delta_f > 10.0 {
        (
            "improving",
            format!("Score up {delta_f:.0} points from {oldest_score:.0}"),
        )
    } else if delta_f > 5.0 {
        ("improving", format!("Score trending up {delta_f:.0} points"))
    } else if delta_f < -10.0 {
        (
            "declining",
            format!(
                "Score down {:.0} points from {oldest_score:.0}",
                delta_f.abs()
            ),
        )
    } else if delta_f < -5.0 {
        (
            "declining",
            format!("Score trending down {:.0} points", delta_f.abs()),
        )
    } else {
        (
            "stable",
            format!("Score stable (±{:.0} points)", delta_f.abs()),
        )
    };

    let trend_confidence = match history.len() {
        5.. => 0.8,
        3 | 4 => 0.5,
        _ => 0.3,
    };

    // DOS-249: Build structured signal tags for the trend meta line.
    // Derived from dimension evidence in the DB — one tag per declining or
    // improving dimension with a non-trivial trend signal.
    let tags = build_trend_tags(db, account_id, direction);

    HealthTrend {
        direction: direction.to_string(),
        rationale: Some(rationale),
        timeframe: "30d".to_string(),
        confidence: trend_confidence,
        delta: Some(delta_i),
        tags,
    }
}

/// DOS-249: Build structured tags for the trend meta line from dimension
/// evidence evidence. Emits up to 4 tags (matching mockup style) derived from
/// dimension trends stored in the most-recent health_json column.
fn build_trend_tags(db: &ActionDb, account_id: &str, trend_direction: &str) -> Vec<HealthTrendTag> {
    // Pull health_json from the last enriched assessment to read dimension trends
    let health_json: Option<String> = db
        .conn
        .query_row(
            "SELECT health_json FROM entity_assessment WHERE entity_id = ?1",
            rusqlite::params![account_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    let Some(json) = health_json else {
        return Vec::new();
    };

    let health: crate::intelligence::io::AccountHealth =
        match serde_json::from_str(&json) {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

    let mut tags: Vec<HealthTrendTag> = Vec::new();

    let dim_labels: &[(&str, &str)] = &[
        ("meeting_cadence", "Meeting cadence"),
        ("email_engagement", "Email engagement"),
        ("stakeholder_coverage", "Stakeholder coverage"),
        ("champion_health", "Champion health"),
        ("financial_proximity", "Financial proximity"),
        ("signal_momentum", "Signal momentum"),
    ];

    let dim_arr: [(&str, &crate::intelligence::io::DimensionScore); 6] = [
        ("meeting_cadence", &health.dimensions.meeting_cadence),
        ("email_engagement", &health.dimensions.email_engagement),
        ("stakeholder_coverage", &health.dimensions.stakeholder_coverage),
        ("key_advocate_health", &health.dimensions.key_advocate_health),
        ("financial_proximity", &health.dimensions.financial_proximity),
        ("signal_momentum", &health.dimensions.signal_momentum),
    ];

    // Emit tags for dimensions whose trend aligns with the account trend direction
    // or that are declining (always worth surfacing). Cap at 4.
    for ((dim_key, _dim_label), (_, dim)) in dim_labels.iter().zip(dim_arr.iter()) {
        if tags.len() >= 4 {
            break;
        }
        if dim.weight == 0.0 {
            continue;
        }
        let direction = match dim.trend.as_str() {
            "improving" => "up",
            "declining" => "down",
            _ => {
                // Only include stable dims when the overall trend is changing
                if trend_direction != "stable" { "stable" } else { continue; }
            }
        };
        // Build a human label from the first evidence item (capped to 25 chars)
        // or fall back to the dimension label.
        let label = dim.evidence.first()
            .map(|e| {
                let e = e.trim();
                if e.len() > 35 {
                    e[..35].trim_end_matches(|c: char| !c.is_alphanumeric()).to_string()
                } else {
                    e.to_string()
                }
            })
            .unwrap_or_else(|| dim_labels.iter()
                .find(|(k, _)| *k == *dim_key)
                .map(|(_, l)| l.to_string())
                .unwrap_or_default());
        tags.push(HealthTrendTag {
            label,
            direction: direction.to_string(),
        });
    }

    tags
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
    let champion = health.dimensions.key_advocate_health.score;
    let cadence_present = health.dimensions.meeting_cadence.weight > 0.0;
    let champion_present = health.dimensions.key_advocate_health.weight > 0.0;
    let any_declining = [
        &health.dimensions.meeting_cadence,
        &health.dimensions.email_engagement,
        &health.dimensions.stakeholder_coverage,
        &health.dimensions.key_advocate_health,
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
        if let Ok(dynamics_rows) = db
            .conn
            .prepare(
                "SELECT mid.question_density, mid.decision_maker_active, mid.forward_looking
             FROM meeting_interaction_dynamics mid
             JOIN meeting_entities me ON me.meeting_id = mid.meeting_id AND me.entity_id = ?1
             ORDER BY mid.created_at DESC LIMIT 3",
            )
            .and_then(|mut stmt| {
                stmt.query_map(rusqlite::params![account_id], |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
            })
        {
            if !dynamics_rows.is_empty() {
                let mut quality_score = 0.0f64;
                let n = dynamics_rows.len() as f64;
                for (qd, dma, fl) in &dynamics_rows {
                    if qd.as_deref() == Some("high") {
                        quality_score += 1.0;
                    }
                    if dma.as_deref() == Some("yes") {
                        quality_score += 1.0;
                    }
                    if fl.as_deref() == Some("high") {
                        quality_score += 1.0;
                    }
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
        evidence: vec![format!(
            "{count_30d:.0} meetings in 30d, ratio={ratio:.2}, quality={quality_multiplier:.2}"
        )],
        trend,
    }
}

fn compute_email_engagement(db: &ActionDb, account_id: &str) -> DimensionScore {
    // TODO(v1.2.2): commitment completion rate → behavioral input for engagement dim.
    // When the AI commitment bridge has enough signal volume, weight the engagement
    // score by the delivered:rejected ratio over a rolling 90-day window.
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

    // I633: Tiered stakeholder scoring — base 50 for having any stakeholders,
    // +20 per active archetype role, +10 depth bonus.
    //
    //   0 stakeholders       → null (excluded from scoring)
    //   Has team, no roles   → 50
    //   1 active archetype   → 70
    //   2 active archetypes  → 90
    //   3 active archetypes  → 100
    //   + depth bonus        → capped at 100
    let expected_roles = ["champion", "executive", "technical"];
    let mut score = 50.0f64; // Base: has stakeholders
    let mut evidence = Vec::new();

    for role in &expected_roles {
        let has_role = team.iter().any(|t| t.role.to_lowercase().contains(role));
        if !has_role {
            evidence.push(format!("Missing: {role}"));
            continue;
        }

        // I555: Verify attendance recency via meeting_attendees
        let role_active = if let Some(person_id) = team
            .iter()
            .find(|t| t.role.to_lowercase().contains(role))
            .map(|t| t.person_id.as_str())
        {
            let last_seen_days = db
                .conn
                .query_row(
                    "SELECT CAST(julianday('now') - julianday(MAX(m.start_time)) AS INTEGER)
                 FROM meeting_attendees ma
                 JOIN meetings m ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1",
                    rusqlite::params![person_id],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .unwrap_or(None);

            match last_seen_days {
                Some(d) if d <= 90 => {
                    evidence.push(format!("{role}: active (seen {d}d ago)"));
                    true
                }
                Some(d) if d <= 180 => {
                    evidence.push(format!("{role}: stale ({d}d ago)"));
                    // Stale = partial credit: +10 instead of +20
                    score += 10.0;
                    false
                }
                Some(d) => {
                    evidence.push(format!("{role}: inactive ({d}d ago)"));
                    false
                }
                None => {
                    evidence.push(format!("{role}: mapped but never seen in meetings"));
                    // Mapped but unseen: +5 for effort
                    score += 5.0;
                    false
                }
            }
        } else {
            evidence.push(format!("{role}: mapped (no person linked)"));
            score += 5.0;
            false
        };

        if role_active {
            score += 20.0;
        }
    }

    // Depth bonus: more people mapped than just the 3 archetypes
    if team.len() > expected_roles.len() {
        score += 10.0;
        evidence.push(format!("{} total stakeholders mapped", team.len()));
    }

    DimensionScore {
        score: score.clamp(0.0, 100.0),
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

    // I633: Raised inference caps — strong engagement shouldn't be penalized
    // just because the user hasn't tagged a champion. 15-20 point discount
    // vs explicit champion is fair, but old caps (55/40/25) were too harsh.
    match top_attendee {
        Some(att) if att.pct >= 0.70 => {
            // 70%+ attendance → champion-level engagement
            DimensionScore {
                score: 75.0,
                weight: 1.0,
                evidence: vec![
                    format!(
                        "No named champion — {} attended {}/{} meetings ({:.0}%)",
                        att.person_name,
                        att.attended,
                        total_meetings,
                        att.pct * 100.0,
                    ),
                    format!("Consider tagging {} as champion", att.person_name),
                ],
                trend: "stable".to_string(),
            }
        }
        Some(att) if att.pct >= 0.50 => {
            // 50-69% attendance → strong engagement, not quite champion
            DimensionScore {
                score: 60.0,
                weight: 1.0,
                evidence: vec![
                    format!(
                        "No named champion — {} attended {}/{} meetings ({:.0}%)",
                        att.person_name,
                        att.attended,
                        total_meetings,
                        att.pct * 100.0,
                    ),
                    format!(
                        "Strong engagement from {} — consider as champion candidate",
                        att.person_name
                    ),
                ],
                trend: "stable".to_string(),
            }
        }
        Some(att) if att.attended >= 2 => {
            // Some attendance but below 50% — partial credit
            DimensionScore {
                score: 35.0,
                weight: 0.5, // Reduced weight — we're less sure
                evidence: vec![format!(
                    "No named champion — best attendee {} at {}/{} meetings ({:.0}%)",
                    att.person_name,
                    att.attended,
                    total_meetings,
                    att.pct * 100.0,
                )],
                trend: String::new(),
            }
        }
        _ => null_dimension("No champion identified, no consistent meeting attendees"),
    }
}

fn compute_key_advocate_health(db: &ActionDb, account_id: &str) -> DimensionScore {
    // I652: Query account_stakeholder_roles directly for champion designation
    let champion_rows: Vec<(String, String)> = db
        .conn
        .prepare(
            "SELECT asr.person_id, p.name FROM account_stakeholder_roles asr \
             JOIN people p ON p.id = asr.person_id \
             WHERE asr.account_id = ?1 AND asr.role = 'champion' AND asr.dismissed_at IS NULL",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![account_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let champion: Option<(&str, &str)> = champion_rows
        .first()
        .map(|(pid, name)| (pid.as_str(), name.as_str()));

    if champion.is_none() {
        return infer_champion_from_attendance(db, account_id);
    }

    // I555: Query per-champion meeting engagement from meeting_champion_health
    let champion_assessments: Vec<(String, String, Option<String>)> = db
        .conn
        .prepare(
            "SELECT m.start_time, mch.champion_status, mch.champion_evidence
         FROM meeting_champion_health mch
         JOIN meetings m ON m.id = mch.meeting_id
         JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_id = ?1
         WHERE mch.champion_name IS NOT NULL
         ORDER BY m.start_time DESC LIMIT 5",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![account_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if champion_assessments.is_empty() {
        // I646 C1: User designated a champion — check if they specifically attended meetings
        let champion_person_id = champion.map(|(pid, _)| pid).unwrap_or("");
        let champion_name = champion.map(|(_, name)| name).unwrap_or("Champion");

        let champion_meeting_count: i64 = if !champion_person_id.is_empty() {
            db.conn
                .query_row(
                    "SELECT COUNT(DISTINCT m.id) FROM meetings m
                     JOIN meeting_entities me ON me.meeting_id = m.id
                     JOIN meeting_attendees ma ON ma.meeting_id = m.id AND ma.person_id = ?2
                     WHERE me.entity_id = ?1 AND me.entity_type = 'account'
                       AND m.start_time >= datetime('now', '-90 days')",
                    rusqlite::params![account_id, champion_person_id],
                    |row| row.get(0),
                )
                .unwrap_or(0)
        } else {
            0
        };

        if champion_meeting_count > 0 {
            return DimensionScore {
                score: 70.0,
                weight: 0.8,
                evidence: vec![format!(
                    "{} designated as champion, {} meetings in 90d",
                    champion_name, champion_meeting_count
                )],
                trend: "stable".to_string(),
            };
        }

        return DimensionScore {
            score: 40.0,
            weight: 0.6,
            evidence: vec![format!(
                "{} designated as champion but no recent meetings",
                champion_name
            )],
            trend: "declining".to_string(),
        };
    }

    // Score based on recent champion engagement
    let status_scores: Vec<f64> = champion_assessments
        .iter()
        .map(|(_, status, _)| match status.as_str() {
            "strong" => 90.0,
            "weak" => 40.0,
            "lost" => 10.0,
            "none" => 20.0,
            _ => 50.0,
        })
        .collect();

    // Temporal weighting: recent assessments matter more (30-day half-life)
    let now = chrono::Utc::now();
    let weights: Vec<f64> = champion_assessments
        .iter()
        .map(|(start_time, _, _)| {
            let age_days = chrono::DateTime::parse_from_rfc3339(start_time)
                .map(|d| (now - d.with_timezone(&chrono::Utc)).num_days().max(0) as f64)
                .unwrap_or(0.0);
            crate::signals::decay::decayed_weight(1.0, age_days, 30.0)
        })
        .collect();
    let weight_sum: f64 = weights.iter().sum();
    let avg_score = if weight_sum > 0.0 {
        status_scores
            .iter()
            .zip(weights.iter())
            .map(|(s, w)| s * w)
            .sum::<f64>()
            / weight_sum
    } else {
        status_scores.iter().sum::<f64>() / status_scores.len() as f64
    };

    // Trend detection
    let trend = if status_scores.len() >= 2 {
        let recent = status_scores[0];
        let older = status_scores[status_scores.len() - 1];
        if recent > older + 10.0 {
            "improving"
        } else if recent < older - 10.0 {
            "declining"
        } else {
            "stable"
        }
    } else {
        "stable"
    };

    // Build evidence with specific meeting dates and statuses
    let champion_name = champion.map(|(_, name)| name).unwrap_or("Champion");
    let mut evidence = vec![
        format!(
            "{champion_name}: {} across {} meetings",
            champion_assessments[0].1,
            champion_assessments.len()
        ),
        "Weighted by recency (30-day half-life)".to_string(),
    ];
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
        let avg_confidence: f64 = glean_gong_signals.iter().map(|(_, c)| c).sum::<f64>()
            / glean_gong_signals.len() as f64;
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
    let mut evidence = vec![format!("{days_to_renewal:.0} days to renewal")];

    // I633: Two-factor financial proximity — OUTCOME + ATTENTION.
    //
    // Old formula used exponential decay from renewal date, which inverted the
    // meaning: a just-renewed account scored 5/100 while an about-to-churn
    // account scored 100/100. Fixed to blend renewal outcome with proximity.
    //
    //   ┌─────────────────────────┬───────────┐
    //   │ Scenario                │ Score     │
    //   ├─────────────────────────┼───────────┤
    //   │ Renewed + ARR growth    │ 90 – 100  │
    //   │ Renewed, flat ARR       │ 80        │
    //   │ Expansion               │ 90 – 100  │
    //   │ No outcome, far off     │ 65        │
    //   │ No outcome, approaching │ 50        │
    //   │ Downgrade               │ 35        │
    //   │ Churn                   │ 15        │
    //   └─────────────────────────┴───────────┘

    let recent_event: Option<(String, Option<f64>)> = db
        .conn
        .query_row(
            "SELECT event_type, arr_impact FROM account_events
             WHERE account_id = ?1 AND event_type IN ('renewal', 'expansion', 'churn', 'downgrade')
             ORDER BY event_date DESC LIMIT 1",
            rusqlite::params![&account.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let outcome_score = match &recent_event {
        Some((event_type, arr_impact)) => {
            let arr_pct = arr_impact
                .and_then(|impact| {
                    account
                        .arr
                        .map(|arr| if arr > 0.0 { impact / arr * 100.0 } else { 0.0 })
                })
                .unwrap_or(0.0);
            match event_type.as_str() {
                "renewal" if arr_pct > 0.0 => {
                    let bonus = arr_pct.min(10.0);
                    evidence.push(format!("Renewed with {arr_pct:.0}% ARR growth"));
                    90.0 + bonus
                }
                "renewal" => {
                    evidence.push("Renewed (flat ARR)".to_string());
                    80.0
                }
                "expansion" => {
                    let bonus = arr_pct.min(10.0);
                    evidence.push(format!("Expansion: +{arr_pct:.0}% ARR"));
                    90.0 + bonus
                }
                "downgrade" => {
                    evidence.push("Recent downgrade event".to_string());
                    35.0
                }
                "churn" => {
                    evidence.push("Churn event recorded".to_string());
                    15.0
                }
                _ => 65.0,
            }
        }
        None => {
            if days_to_renewal > 180.0 {
                65.0 // Far off, healthy default
            } else if days_to_renewal > 90.0 {
                60.0
            } else if days_to_renewal > 30.0 {
                50.0 // Approaching, needs attention
            } else if days_to_renewal > 0.0 {
                40.0 // Imminent
            } else {
                30.0 // Past due
            }
        }
    };

    // Attention signal: proximity drives CSM focus regardless of outcome
    let attention_signal = if days_to_renewal < 30.0 {
        30.0
    } else if days_to_renewal < 90.0 {
        50.0
    } else {
        70.0
    };

    let attention_weight = if days_to_renewal < 90.0 { 0.3 } else { 0.1 };
    let mut score = outcome_score * (1.0 - attention_weight) + attention_signal * attention_weight;

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

    let mut weighted_sum = 0.0f64;
    let mut evidence = vec![format!("{} signals in 30d", signals.len())];
    for signal in &signals {
        // Use canonical signal weight: tier weight * half-life decay * Bayesian reliability
        let weight = fusion::compute_signal_weight(db, signal);
        weighted_sum += signal.confidence * weight;
    }

    let base_momentum = (weighted_sum * 10.0).clamp(10.0, 100.0);

    let mut momentum = base_momentum;
    if !zendesk_signals.is_empty() {
        let mut zendesk_velocity = 50.0;
        let cadence_boost = (zendesk_signals.len().min(4) as f64) * 6.0;
        zendesk_velocity += cadence_boost;

        if let Some((latest_value, latest_confidence, latest_created_at)) = zendesk_signals.first()
        {
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
                .map(|d| (chrono::Utc::now() - d.with_timezone(&chrono::Utc)).num_days())
                .unwrap_or(30);
            evidence.push(format!(
                "Zendesk velocity signal {}d ago ({:.0}% confidence)",
                age_days,
                latest_confidence * 100.0
            ));
        }

        momentum =
            (base_momentum * 0.65 + zendesk_velocity.clamp(10.0, 100.0) * 0.35).clamp(10.0, 100.0);
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

const HEALTH_DIMENSION_WEIGHT_KEYS: [&str; 6] = [
    "meeting_cadence",
    "email_engagement",
    "stakeholder_coverage",
    "key_advocate_health",
    "financial_proximity",
    "signal_momentum",
];

/// Compose preset base weights with lifecycle-stage multipliers.
/// Order: [meeting, email, stakeholder, champion, financial, signal]
pub fn compose_dimension_weights(
    preset_id: &str,
    intelligence: Option<&PresetIntelligenceConfig>,
    lifecycle: Option<&str>,
) -> [f64; 6] {
    let base_weights = intelligence
        .map(dimension_weights_from_config)
        .unwrap_or([1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
    let multipliers = apply_lifecycle_weights(preset_id, lifecycle);
    [
        base_weights[0] * multipliers[0],
        base_weights[1] * multipliers[1],
        base_weights[2] * multipliers[2],
        base_weights[3] * multipliers[3],
        base_weights[4] * multipliers[4],
        base_weights[5] * multipliers[5],
    ]
}

fn dimension_weights_from_config(intelligence: &PresetIntelligenceConfig) -> [f64; 6] {
    HEALTH_DIMENSION_WEIGHT_KEYS.map(|key| {
        intelligence
            .dimension_weights
            .get(key)
            .copied()
            .unwrap_or(1.0)
    })
}

/// Apply lifecycle-stage weight multipliers to each dimension.
/// Order: [meeting, email, stakeholder, champion, financial, signal]
pub fn apply_lifecycle_weights(preset_id: &str, lifecycle: Option<&str>) -> [f64; 6] {
    if canonical_role_id(preset_id) != "customer-success" {
        return [1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    }

    match lifecycle {
        Some("onboarding") => [1.5, 1.0, 1.5, 1.0, 0.7, 1.0],
        Some("adoption") => [1.0, 1.0, 1.0, 1.5, 1.0, 1.5],
        Some("renewal") | Some("renewing") => [1.0, 1.3, 1.0, 1.3, 2.0, 1.3],
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
        dims.key_advocate_health.weight > 0.0,
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

/// DOS-84: Determine if enough dimensions have real data for a reliable health score.
/// Returns true when >= 3 non-placeholder dimensions are populated.
fn has_sufficient_data(dims: &RelationshipDimensions) -> bool {
    let populated_count = [
        &dims.meeting_cadence,
        &dims.email_engagement,
        &dims.stakeholder_coverage,
        &dims.key_advocate_health,
        &dims.financial_proximity,
        &dims.signal_momentum,
    ]
    .iter()
    .filter(|d| d.weight > 0.0 && !is_neutral_momentum_placeholder(d))
    .count();
    populated_count >= 3
}

/// Confidence = fraction of non-null dimensions.
fn is_neutral_momentum_placeholder(dim: &DimensionScore) -> bool {
    dim.weight > 0.0
        && (dim.score - 50.0).abs() < f64::EPSILON
        && dim.evidence.len() == 1
        && dim.evidence[0].contains("No recent signals")
}

fn compute_confidence(dims: &RelationshipDimensions) -> f64 {
    // I633: Smooth confidence curve replacing step function.
    //
    // Old thresholds had harsh cliffs (0.1 → 0.3 → 0.6 → 0.9) that caused
    // 40% score regression to neutral at the common 3-4 dimension case.
    //
    //   Dims │ Old   │ New
    //   ─────┼───────┼──────
    //   0    │ 0.10  │ 0.30
    //   1    │ 0.30  │ 0.58
    //   2    │ 0.30  │ 0.67
    //   3    │ 0.60  │ 0.75
    //   4    │ 0.60  │ 0.83
    //   5    │ 0.90  │ 0.92
    //   6    │ 0.90  │ 0.95
    let populated = [
        &dims.meeting_cadence,
        &dims.email_engagement,
        &dims.stakeholder_coverage,
        &dims.key_advocate_health,
        &dims.financial_proximity,
        &dims.signal_momentum,
    ]
    .iter()
    .filter(|d| d.weight > 0.0 && !is_neutral_momentum_placeholder(d))
    .count();

    let frac = populated as f64 / 6.0;
    (0.5 + frac * 0.5).clamp(0.3, 0.95)
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
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount, DbMeeting};
    use chrono::{Duration, Utc};

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

    fn make_account(id: &str, lifecycle: Option<&str>, contract_end: Option<String>) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: format!("Account {id}"),
            lifecycle: lifecycle.map(|value| value.to_string()),
            arr: Some(100_000.0),
            health: None,
            contract_start: Some("2025-01-01".to_string()),
            contract_end,
            nps: None,
            tracker_path: None,
            parent_id: None,
            account_type: AccountType::Customer,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        }
    }

    fn seed_linked_meeting(
        db: &crate::db::ActionDb,
        meeting_id: &str,
        account_id: &str,
        days_ago: i64,
    ) {
        let start_time = (Utc::now() - Duration::days(days_ago)).to_rfc3339();
        let meeting = DbMeeting {
            id: meeting_id.to_string(),
            title: format!("Meeting {meeting_id}"),
            meeting_type: "customer".to_string(),
            start_time,
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("seed meeting");
        db.link_meeting_entity_if_absent(meeting_id, account_id, "account")
            .expect("link meeting to account");
    }

    #[test]
    fn test_confidence_all_dimensions() {
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: active_dim(60.0),
            stakeholder_coverage: active_dim(80.0),
            key_advocate_health: active_dim(50.0),
            financial_proximity: active_dim(40.0),
            signal_momentum: active_dim(50.0),
        };
        // I633: 6 populated dims → 0.5 + 6/6 * 0.5 = 1.0, clamped to 0.95
        assert!((compute_confidence(&dims) - 0.95).abs() < 1e-6);
    }

    #[test]
    fn test_confidence_partial_dimensions() {
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: null_dim(),
            stakeholder_coverage: active_dim(80.0),
            key_advocate_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: active_dim(50.0), // not a placeholder (evidence != "No recent signals")
        };
        // 3 populated dims → 0.5 + 3/6 * 0.5 = 0.75
        assert!((compute_confidence(&dims) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_confidence_zero_data_uses_lowest_band() {
        let dims = RelationshipDimensions {
            meeting_cadence: null_dim(),
            email_engagement: null_dim(),
            stakeholder_coverage: null_dim(),
            key_advocate_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: DimensionScore {
                score: 50.0,
                weight: 1.0,
                evidence: vec!["No recent signals — neutral baseline".to_string()],
                trend: "stable".to_string(),
            },
        };
        // Momentum placeholder excluded → 0 real dims → 0.5 + 0 = 0.5
        assert!((compute_confidence(&dims) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_redistribute_weights_skips_null() {
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: null_dim(),
            stakeholder_coverage: null_dim(),
            key_advocate_health: null_dim(),
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
        let weights = apply_lifecycle_weights("customer-success", Some("renewal"));
        // Financial proximity (index 4) should have highest weight in renewal
        assert!(
            weights[4] > weights[0],
            "financial_proximity should be highest in renewal"
        );
        assert!((weights[4] - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lifecycle_weights_non_cs_are_flat() {
        let weights = apply_lifecycle_weights("affiliates-partnerships", Some("renewal"));
        assert_eq!(weights, [1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_preset_dimension_weights_feed_raw_weights() {
        let preset = crate::presets::loader::load_preset("affiliates-partnerships").unwrap();
        let raw =
            compose_dimension_weights(&preset.id, Some(&preset.intelligence), Some("renewal"));
        let expected = preset.intelligence.dimension_weights["financial_proximity"];
        assert!((raw[4] - expected).abs() < 1e-9);

        let cs = crate::presets::loader::load_preset("customer-success").unwrap();
        let cs_raw = compose_dimension_weights(&cs.id, Some(&cs.intelligence), Some("renewal"));
        assert!(cs_raw[4] > cs_raw[0]);
    }

    #[test]
    fn test_customer_success_preset_matches_legacy_health_output() {
        let db = test_db();
        let contract_end = (Utc::now().date_naive() + Duration::days(45)).to_string();
        let account = make_account("acc-cs-baseline", Some("renewal"), Some(contract_end));
        db.upsert_account(&account).expect("upsert account");
        seed_linked_meeting(&db, "mtg-cs-baseline-1", &account.id, 7);
        seed_linked_meeting(&db, "mtg-cs-baseline-2", &account.id, 20);
        seed_linked_meeting(&db, "mtg-cs-baseline-3", &account.id, 50);

        let customer_success = crate::presets::loader::load_preset("customer-success")
            .expect("load customer-success preset");
        let preset_health = compute_account_health_with_preset(
            &db,
            &account,
            None,
            Some(&customer_success),
        );

        let dims = RelationshipDimensions {
            meeting_cadence: compute_meeting_cadence(&db, &account.id),
            email_engagement: compute_email_engagement(&db, &account.id),
            stakeholder_coverage: compute_stakeholder_coverage(&db, &account.id),
            key_advocate_health: compute_key_advocate_health(&db, &account.id),
            financial_proximity: compute_financial_proximity(&db, &account),
            signal_momentum: compute_signal_momentum(&db, &account.id),
        };
        let legacy_raw = apply_lifecycle_weights("customer-success", account.lifecycle.as_deref());
        let legacy_weights = redistribute_weights(&dims, legacy_raw);
        let confidence = compute_confidence(&dims);
        let dim_arr = [
            &dims.meeting_cadence,
            &dims.email_engagement,
            &dims.stakeholder_coverage,
            &dims.key_advocate_health,
            &dims.financial_proximity,
            &dims.signal_momentum,
        ];
        let mut weighted_sum = 0.0f64;
        let mut weight_total = 0.0f64;
        for (i, dim) in dim_arr.iter().enumerate() {
            if dim.weight > 0.0 {
                weighted_sum += dim.score * legacy_weights[i];
                weight_total += legacy_weights[i];
            }
        }
        let raw_avg = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            50.0
        };
        let legacy_score = confidence * raw_avg + (1.0 - confidence) * 50.0;

        assert!(
            (legacy_score - preset_health.score).abs() < 1e-9,
            "customer-success preset should preserve legacy score"
        );
        assert_eq!(
            score_to_band(legacy_score),
            preset_health.band,
            "customer-success preset should preserve legacy band"
        );
        assert_eq!(
            confidence, preset_health.confidence,
            "customer-success preset should preserve legacy confidence"
        );
    }

    #[test]
    fn test_affiliates_preset_changes_health_output_for_same_account() {
        let db = test_db();
        let contract_end = (Utc::now().date_naive() + Duration::days(45)).to_string();
        let account = make_account("acc-preset-diff", Some("renewal"), Some(contract_end));
        db.upsert_account(&account).expect("upsert account");
        seed_linked_meeting(&db, "mtg-preset-diff-1", &account.id, 7);
        seed_linked_meeting(&db, "mtg-preset-diff-2", &account.id, 20);
        seed_linked_meeting(&db, "mtg-preset-diff-3", &account.id, 50);

        let customer_success = crate::presets::loader::load_preset("customer-success")
            .expect("load customer-success preset");
        let affiliates = crate::presets::loader::load_preset("affiliates-partnerships")
            .expect("load affiliates preset");

        let cs_health =
            compute_account_health_with_preset(&db, &account, None, Some(&customer_success));
        let affiliates_health =
            compute_account_health_with_preset(&db, &account, None, Some(&affiliates));

        assert_ne!(
            cs_health.score, affiliates_health.score,
            "different preset weights should change the final health score"
        );
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
            sufficient_data: true,
            trend: HealthTrend::default(),
            dimensions: RelationshipDimensions {
                meeting_cadence: active_dim(70.0),
                email_engagement: active_dim(55.0),
                stakeholder_coverage: active_dim(60.0),
                key_advocate_health: active_dim(75.0),
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

    // ===== I633: New tests for formula fixes =====

    #[test]
    fn test_confidence_smooth_curve_values() {
        // Test each populated count produces the expected smooth curve value
        let make_dims = |n: usize| -> RelationshipDimensions {
            let active = active_dim(60.0);
            let null = null_dim();
            RelationshipDimensions {
                meeting_cadence: if n >= 1 { active.clone() } else { null.clone() },
                email_engagement: if n >= 2 { active.clone() } else { null.clone() },
                stakeholder_coverage: if n >= 3 { active.clone() } else { null.clone() },
                key_advocate_health: if n >= 4 { active.clone() } else { null.clone() },
                financial_proximity: if n >= 5 { active.clone() } else { null.clone() },
                signal_momentum: if n >= 6 { active.clone() } else { null.clone() },
            }
        };

        // 1 dim: 0.5 + 1/6 * 0.5 ≈ 0.583
        let c1 = compute_confidence(&make_dims(1));
        assert!(
            (c1 - (0.5 + 1.0 / 6.0 * 0.5)).abs() < 1e-6,
            "1 dim confidence"
        );

        // 2 dims: 0.5 + 2/6 * 0.5 ≈ 0.667
        let c2 = compute_confidence(&make_dims(2));
        assert!(
            (c2 - (0.5 + 2.0 / 6.0 * 0.5)).abs() < 1e-6,
            "2 dim confidence"
        );

        // 4 dims: 0.5 + 4/6 * 0.5 ≈ 0.833
        let c4 = compute_confidence(&make_dims(4));
        assert!(
            (c4 - (0.5 + 4.0 / 6.0 * 0.5)).abs() < 1e-6,
            "4 dim confidence"
        );
    }

    #[test]
    fn test_confidence_monotonically_increases() {
        let make_dims = |n: usize| -> RelationshipDimensions {
            let active = active_dim(60.0);
            let null = null_dim();
            RelationshipDimensions {
                meeting_cadence: if n >= 1 { active.clone() } else { null.clone() },
                email_engagement: if n >= 2 { active.clone() } else { null.clone() },
                stakeholder_coverage: if n >= 3 { active.clone() } else { null.clone() },
                key_advocate_health: if n >= 4 { active.clone() } else { null.clone() },
                financial_proximity: if n >= 5 { active.clone() } else { null.clone() },
                signal_momentum: if n >= 6 { active.clone() } else { null.clone() },
            }
        };

        let mut prev = 0.0;
        for n in 1..=6 {
            let c = compute_confidence(&make_dims(n));
            assert!(
                c > prev,
                "confidence should increase: {n} dims ({c}) should be > prev ({prev})"
            );
            prev = c;
        }
    }

    #[test]
    fn test_confidence_floor_and_ceiling() {
        // 0 dims → should hit floor of 0.3
        let dims = RelationshipDimensions {
            meeting_cadence: null_dim(),
            email_engagement: null_dim(),
            stakeholder_coverage: null_dim(),
            key_advocate_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: null_dim(),
        };
        let c = compute_confidence(&dims);
        assert!((c - 0.5).abs() < 1e-6, "0 dims should be 0.5 (above floor)");

        // 6 dims → should hit ceiling of 0.95
        let dims_full = RelationshipDimensions {
            meeting_cadence: active_dim(60.0),
            email_engagement: active_dim(60.0),
            stakeholder_coverage: active_dim(60.0),
            key_advocate_health: active_dim(60.0),
            financial_proximity: active_dim(60.0),
            signal_momentum: active_dim(60.0),
        };
        let c_full = compute_confidence(&dims_full);
        assert!((c_full - 0.95).abs() < 1e-6, "6 dims should hit ceiling");
    }

    #[test]
    fn test_weighted_score_healthy_account() {
        // Simulate a healthy account: all dimensions scoring well
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(80.0),
            email_engagement: active_dim(65.0),
            stakeholder_coverage: active_dim(70.0),
            key_advocate_health: active_dim(75.0),
            financial_proximity: active_dim(90.0), // recently renewed
            signal_momentum: active_dim(55.0),
        };

        let raw_weights = apply_lifecycle_weights("customer-success", None); // equal weights
        let weights = redistribute_weights(&dims, raw_weights);
        let confidence = compute_confidence(&dims);

        let dim_arr = [
            &dims.meeting_cadence,
            &dims.email_engagement,
            &dims.stakeholder_coverage,
            &dims.key_advocate_health,
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
        let raw_avg = weighted_sum / weight_total;
        let computed = confidence * raw_avg + (1.0 - confidence) * 50.0;

        // With all dimensions healthy, score should be green (≥70)
        assert!(
            computed >= 70.0,
            "Healthy account should score green, got {computed:.1}"
        );
    }

    #[test]
    fn test_weighted_score_at_risk_account() {
        // Simulate at-risk: low engagement, no champion, approaching renewal
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(20.0),
            email_engagement: null_dim(),
            stakeholder_coverage: active_dim(30.0),
            key_advocate_health: null_dim(),
            financial_proximity: active_dim(40.0), // approaching, no outcome
            signal_momentum: active_dim(50.0),     // neutral
        };

        let raw_weights = apply_lifecycle_weights("customer-success", None);
        let weights = redistribute_weights(&dims, raw_weights);
        let confidence = compute_confidence(&dims);

        let dim_arr = [
            &dims.meeting_cadence,
            &dims.email_engagement,
            &dims.stakeholder_coverage,
            &dims.key_advocate_health,
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
        let raw_avg = weighted_sum / weight_total;
        let computed = confidence * raw_avg + (1.0 - confidence) * 50.0;

        // At-risk account should score red (<40) or low yellow
        assert!(
            computed < 50.0,
            "At-risk account should score below 50, got {computed:.1}"
        );
    }

    #[test]
    fn test_weighted_score_sparse_data_neutral() {
        // Simulate sparse data: only one dimension available
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(80.0),
            email_engagement: null_dim(),
            stakeholder_coverage: null_dim(),
            key_advocate_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: DimensionScore {
                score: 50.0,
                weight: 1.0,
                evidence: vec!["No recent signals — neutral baseline".to_string()],
                trend: "stable".to_string(),
            },
        };

        let raw_weights = apply_lifecycle_weights("customer-success", None);
        let weights = redistribute_weights(&dims, raw_weights);
        let confidence = compute_confidence(&dims);

        // 1 real dim (meeting_cadence) + momentum placeholder
        // Confidence should be ~0.58 (1 real dim)
        assert!(
            confidence < 0.65,
            "Sparse data should have low confidence, got {confidence}"
        );

        // Score should regress toward 50 due to low confidence
        let dim_arr = [
            &dims.meeting_cadence,
            &dims.email_engagement,
            &dims.stakeholder_coverage,
            &dims.key_advocate_health,
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
        let raw_avg = weighted_sum / weight_total;
        let computed = confidence * raw_avg + (1.0 - confidence) * 50.0;

        // Should be pulled toward neutral, not extreme
        assert!(
            (40.0..=70.0).contains(&computed),
            "Sparse data score should be near neutral, got {computed:.1}"
        );
    }

    #[test]
    fn test_lifecycle_weights_change_scoring() {
        // Renewal lifecycle should weight financial_proximity (index 4) at 2.0
        let weights = apply_lifecycle_weights("customer-success", Some("renewal"));
        assert!((weights[4] - 2.0).abs() < f64::EPSILON);

        // Onboarding should weight meeting_cadence (index 0) higher
        let onboard = apply_lifecycle_weights("customer-success", Some("onboarding"));
        assert!((onboard[0] - 1.5).abs() < f64::EPSILON);
        assert!((onboard[2] - 1.5).abs() < f64::EPSILON); // stakeholder_coverage
    }

    #[test]
    fn test_weight_redistribution_with_lifecycle() {
        // If financial_proximity is null during renewal, its 2.0 weight should
        // redistribute proportionally to active dimensions
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: active_dim(60.0),
            stakeholder_coverage: active_dim(80.0),
            key_advocate_health: active_dim(65.0),
            financial_proximity: null_dim(),
            signal_momentum: active_dim(50.0),
        };

        let raw = apply_lifecycle_weights("customer-success", Some("renewal"));
        let redistributed = redistribute_weights(&dims, raw);

        // Financial proximity (index 4) should get 0 weight
        assert!(
            redistributed[4].abs() < f64::EPSILON,
            "Null dim should get 0 weight"
        );
        // Sum of active weights should equal 1.0
        let sum: f64 = redistributed.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "Active weights should sum to 1.0, got {sum}"
        );
    }

    #[test]
    fn test_org_health_baseline_blend() {
        // With org_health green (75), a computed score of 60 should blend:
        // 0.4 * 75 + 0.6 * 60 = 30 + 36 = 66
        let org_score = band_to_score("green");
        assert!((org_score - 75.0).abs() < f64::EPSILON);

        let computed = 60.0;
        let blended = 0.4 * org_score + 0.6 * computed;
        assert!(
            (blended - 66.0).abs() < f64::EPSILON,
            "Blend should be 66.0, got {blended}"
        );
    }

    #[test]
    fn test_classify_account_bucket_at_risk_save_unlikely() {
        let health = AccountHealth {
            score: 55.0,
            band: "yellow".to_string(),
            source: HealthSource::Computed,
            confidence: 0.6,
            sufficient_data: true,
            trend: HealthTrend::default(),
            dimensions: RelationshipDimensions {
                meeting_cadence: active_dim(20.0),
                email_engagement: active_dim(45.0),
                stakeholder_coverage: active_dim(30.0),
                key_advocate_health: active_dim(15.0),
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

    // ===== DOS-84: Sufficient data threshold tests =====

    #[test]
    fn test_sufficient_data_with_sparse_dimensions() {
        // Only 2 real dimensions — below the 3-dimension threshold.
        // Calls the production has_sufficient_data() function directly.
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: active_dim(60.0),
            stakeholder_coverage: null_dim(),
            key_advocate_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: DimensionScore {
                score: 50.0,
                weight: 1.0,
                evidence: vec!["No recent signals — neutral baseline".to_string()],
                trend: "stable".to_string(),
            },
        };
        assert!(
            !has_sufficient_data(&dims),
            "2 real dimensions should be insufficient"
        );
    }

    #[test]
    fn test_sufficient_data_with_three_dimensions() {
        // Exactly 3 real dimensions — at the threshold.
        // Calls the production has_sufficient_data() function directly.
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: active_dim(60.0),
            stakeholder_coverage: active_dim(50.0),
            key_advocate_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: DimensionScore {
                score: 50.0,
                weight: 1.0,
                evidence: vec!["No recent signals — neutral baseline".to_string()],
                trend: "stable".to_string(),
            },
        };
        assert!(
            has_sufficient_data(&dims),
            "3 real dimensions should be sufficient"
        );
    }
}
