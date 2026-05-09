use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};

use crate::abilities::provenance::{
    DataSource, EmailId, EntityId, MeetingId, SourceIdentifier, SourceRef,
};
use crate::abilities::temporal::{
    DataPoint, DetectRoleChangeInput, DetectRoleChangeResult, EngagementWindow,
    RefreshEngagementCurveInput, RefreshEngagementCurveResult, RoleEntry, TrajectoryBundle,
    TrajectoryKind, TrajectoryQueryDepth, TrajectorySnapshot, RETENTION_DAYS,
};
use crate::db::ActionDb;

const SECONDS_PER_WEEK: i64 = 7 * 24 * 60 * 60;
const ENGAGEMENT_BACKFILL_CHUNK_WEEKS: usize = 32;
const REFRESH_ENGAGEMENT_CURVE_ABILITY_ID: &str = "refresh_engagement_curve";

pub fn read_trajectory_bundle_from_db(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    depth: TrajectoryQueryDepth,
    computed_at: DateTime<Utc>,
) -> Result<TrajectoryBundle, String> {
    let limit = depth.limit();
    if limit == 0 {
        return Ok(TrajectoryBundle::default());
    }

    Ok(TrajectoryBundle {
        engagement_curve: read_engagement_curve(db, entity_type, entity_id, limit, computed_at)?,
        role_progression: read_role_progression(db, entity_type, entity_id, limit, computed_at)?,
    })
}

pub fn refresh_engagement_curve_in_db(
    db: &ActionDb,
    input: RefreshEngagementCurveInput,
    computed_at: DateTime<Utc>,
) -> Result<RefreshEngagementCurveResult, String> {
    let entity_type = normalize_required(input.entity_type, "entity_type")?;
    let latest_week_start = complete_week_start(computed_at);
    let retention_cutoff = computed_at - Duration::days(i64::from(RETENTION_DAYS));
    let refresh_plan = engagement_rows_to_refresh(
        db,
        &entity_type,
        &input.entity_id,
        latest_week_start,
        retention_cutoff,
    )?;

    db.with_transaction(|tx| {
        for row in &refresh_plan.rows {
            tx.conn_ref()
                .execute(
                    "INSERT INTO entity_engagement_curve (
                         entity_type, entity_id, week_start, meetings_count, emails_count,
                         bidirectional_ratio, source_refs_json
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(entity_type, entity_id, week_start) DO UPDATE SET
                         meetings_count = excluded.meetings_count,
                         emails_count = excluded.emails_count,
                         bidirectional_ratio = excluded.bidirectional_ratio,
                         source_refs_json = excluded.source_refs_json,
                         source_invalidated_at = COALESCE(
                             excluded.source_invalidated_at,
                             entity_engagement_curve.source_invalidated_at
                         )",
                    params![
                        &entity_type,
                        &input.entity_id,
                        row.week_start.to_rfc3339(),
                        i64::from(row.meetings_count),
                        i64::from(row.emails_count),
                        row.bidirectional_ratio,
                        &row.source_refs_json,
                    ],
                )
                .map_err(|error| format!("upsert entity_engagement_curve: {error}"))?;
        }

        tx.conn_ref()
            .execute(
                "DELETE FROM entity_engagement_curve
                 WHERE entity_type = ?1
                   AND entity_id = ?2
                   AND datetime(week_start) < datetime(?3)",
                params![
                    &entity_type,
                    &input.entity_id,
                    retention_cutoff.to_rfc3339()
                ],
            )
            .map_err(|error| format!("prune entity_engagement_curve: {error}"))?;

        if let Some(last_completed_week_start) = refresh_plan.last_completed_week_start {
            tx.conn_ref()
                .execute(
                    "INSERT INTO temporal_backfill_state (
                         entity_type, entity_id, ability_id, last_completed_week_start,
                         retention_cutoff, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(entity_type, entity_id, ability_id) DO UPDATE SET
                         last_completed_week_start = excluded.last_completed_week_start,
                         retention_cutoff = excluded.retention_cutoff,
                         updated_at = excluded.updated_at",
                    params![
                        &entity_type,
                        &input.entity_id,
                        REFRESH_ENGAGEMENT_CURVE_ABILITY_ID,
                        last_completed_week_start.to_rfc3339(),
                        retention_cutoff.to_rfc3339(),
                        computed_at.to_rfc3339(),
                    ],
                )
                .map_err(|error| format!("upsert temporal_backfill_state: {error}"))?;
        }

        Ok(())
    })?;

    let retained_weeks = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*)
             FROM entity_engagement_curve
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND source_invalidated_at IS NULL",
            params![&entity_type, &input.entity_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("count retained entity_engagement_curve rows: {error}"))
        .and_then(u16_from_i64)?;

    Ok(RefreshEngagementCurveResult {
        entity_type,
        entity_id: input.entity_id,
        week_start: latest_week_start,
        rows_written: u32::try_from(refresh_plan.rows.len())
            .map_err(|_| "engagement row count does not fit u32".to_string())?,
        retained_weeks,
        computed_at,
    })
}

pub fn detect_role_change_in_db(
    db: &ActionDb,
    input: DetectRoleChangeInput,
    computed_at: DateTime<Utc>,
) -> Result<DetectRoleChangeResult, String> {
    let entity_type = normalize_required(input.entity_type, "entity_type")?;
    let observed_at = input.observed_at.unwrap_or(computed_at);
    let title = normalize_required(input.title, "title")?;
    let org = normalize_optional(input.org);
    let seniority = normalize_optional(input.seniority);
    let source_refs_json = serde_json::to_string(&input.source_refs)
        .map_err(|error| format!("serialize role source refs: {error}"))?;

    db.with_transaction(|tx| {
        if let Some(existing) =
            role_entry_starting_at(tx, &entity_type, &input.entity_id, observed_at)?
        {
            if existing.title == title
                && existing.org == org
                && existing.seniority == seniority
                && existing.source_refs_json == source_refs_json
            {
                return Ok(DetectRoleChangeResult {
                    entity_type,
                    entity_id: input.entity_id,
                    appended: false,
                    current_started_at: observed_at,
                    prior_ended_at: None,
                    computed_at,
                });
            }

            return Err(format!(
                "duplicate role progression observed_at `{}` for entity `{}`",
                observed_at.to_rfc3339(),
                input.entity_id
            ));
        }

        let active_entry = active_role_entry_at(tx, &entity_type, &input.entity_id, observed_at)?;
        if let Some(entry) = active_entry.as_ref().filter(|entry| {
            entry.title == title && entry.org == org && entry.seniority == seniority
        }) {
            return Ok(DetectRoleChangeResult {
                entity_type,
                entity_id: input.entity_id,
                appended: false,
                current_started_at: entry.started_at,
                prior_ended_at: None,
                computed_at,
            });
        }

        let inserted_ended_at =
            active_entry
                .as_ref()
                .and_then(|entry| entry.ended_at)
                .or(next_role_started_at(
                    tx,
                    &entity_type,
                    &input.entity_id,
                    observed_at,
                )?);
        let closed_prior = active_entry.is_some();

        tx.conn_ref()
            .execute(
                "UPDATE person_role_progression
                 SET ended_at = ?3
                 WHERE entity_type = ?1
                   AND entity_id = ?2
                   AND datetime(started_at) <= datetime(?3)
                   AND (ended_at IS NULL OR datetime(ended_at) > datetime(?3))
                   AND source_invalidated_at IS NULL",
                params![&entity_type, &input.entity_id, observed_at.to_rfc3339()],
            )
            .map_err(|error| format!("close prior person_role_progression row: {error}"))?;

        tx.conn_ref()
            .execute(
                "INSERT INTO person_role_progression (
                     entity_type, entity_id, started_at, ended_at, title, org, seniority,
                     source_refs_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &entity_type,
                    &input.entity_id,
                    observed_at.to_rfc3339(),
                    inserted_ended_at.map(|value| value.to_rfc3339()),
                    &title,
                    org.as_deref(),
                    seniority.as_deref(),
                    source_refs_json,
                ],
            )
            .map_err(|error| format!("insert person_role_progression row: {error}"))?;

        Ok(DetectRoleChangeResult {
            entity_type,
            entity_id: input.entity_id,
            appended: true,
            current_started_at: observed_at,
            prior_ended_at: closed_prior.then_some(observed_at),
            computed_at,
        })
    })
}

fn read_engagement_curve(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    limit: usize,
    computed_at: DateTime<Utc>,
) -> Result<Option<TrajectorySnapshot<EngagementWindow>>, String> {
    if !table_exists(db, "entity_engagement_curve")? {
        return Ok(None);
    }

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT week_start, meetings_count, emails_count,
                    bidirectional_ratio, source_refs_json
             FROM entity_engagement_curve
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND source_invalidated_at IS NULL
             ORDER BY week_start DESC
             LIMIT ?3",
        )
        .map_err(|error| format!("prepare entity_engagement_curve read: {error}"))?;

    let rows = stmt
        .query_map(
            params![entity_type, entity_id, i64_from_usize(limit)?],
            |row| {
                let at_raw: String = row.get(0)?;
                let meetings_count: i64 = row.get(1)?;
                let emails_count: i64 = row.get(2)?;
                let bidirectional_ratio: f32 = row.get(3)?;
                let source_refs_json: String = row.get(4)?;
                Ok((
                    at_raw,
                    meetings_count,
                    emails_count,
                    bidirectional_ratio,
                    source_refs_json,
                ))
            },
        )
        .map_err(|error| format!("query entity_engagement_curve: {error}"))?;

    let mut series = Vec::new();
    for row in rows {
        let (at_raw, meetings_count, emails_count, ratio, source_refs_json) =
            row.map_err(|error| format!("map entity_engagement_curve row: {error}"))?;
        series.push(DataPoint {
            at: parse_db_datetime(&at_raw)?,
            value: EngagementWindow::new(
                u32_from_i64(meetings_count)?,
                u32_from_i64(emails_count)?,
                ratio,
            )
            .map_err(|error| format!("invalid engagement row: {error}"))?,
            source_refs: parse_source_refs(&source_refs_json),
        });
    }

    if series.is_empty() {
        return Ok(None);
    }

    TrajectorySnapshot::new(
        TrajectoryKind::EngagementCurve,
        entity_type.to_string(),
        EntityId::new(entity_id.to_string()),
        series,
        computed_at,
        1.0,
    )
    .map(Some)
    .map_err(|error| format!("invalid engagement snapshot: {error}"))
}

fn read_role_progression(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    limit: usize,
    computed_at: DateTime<Utc>,
) -> Result<Option<TrajectorySnapshot<RoleEntry>>, String> {
    if !table_exists(db, "person_role_progression")? {
        return Ok(None);
    }

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT started_at, ended_at, title, org, seniority, source_refs_json
             FROM person_role_progression
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND source_invalidated_at IS NULL
             ORDER BY started_at DESC
             LIMIT ?3",
        )
        .map_err(|error| format!("prepare person_role_progression read: {error}"))?;

    let rows = stmt
        .query_map(
            params![entity_type, entity_id, i64_from_usize(limit)?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .map_err(|error| format!("query person_role_progression: {error}"))?;

    let mut series = Vec::new();
    for row in rows {
        let (started_at_raw, ended_at_raw, title, org, seniority, source_refs_json) =
            row.map_err(|error| format!("map person_role_progression row: {error}"))?;
        let started_at = parse_db_datetime(&started_at_raw)?;
        let ended_at = ended_at_raw.as_deref().map(parse_db_datetime).transpose()?;
        series.push(DataPoint {
            at: started_at,
            value: RoleEntry {
                started_at,
                ended_at,
                title,
                org,
                seniority,
            },
            source_refs: parse_source_refs(&source_refs_json),
        });
    }

    if series.is_empty() {
        return Ok(None);
    }

    TrajectorySnapshot::new(
        TrajectoryKind::RoleProgression,
        entity_type.to_string(),
        EntityId::new(entity_id.to_string()),
        series,
        computed_at,
        1.0,
    )
    .map(Some)
    .map_err(|error| format!("invalid role snapshot: {error}"))
}

struct StoredRoleEntry {
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    title: String,
    org: Option<String>,
    seniority: Option<String>,
    source_refs_json: String,
}

fn role_entry_starting_at(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    observed_at: DateTime<Utc>,
) -> Result<Option<StoredRoleEntry>, String> {
    if !table_exists(db, "person_role_progression")? {
        return Ok(None);
    }

    db.conn_ref()
        .query_row(
            "SELECT started_at, ended_at, title, org, seniority, source_refs_json
             FROM person_role_progression
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND started_at = ?3
               AND source_invalidated_at IS NULL
             LIMIT 1",
            params![entity_type, entity_id, observed_at.to_rfc3339()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("query matching person_role_progression row: {error}"))?
        .map(stored_role_entry_from_row)
        .transpose()
}

fn active_role_entry_at(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    observed_at: DateTime<Utc>,
) -> Result<Option<StoredRoleEntry>, String> {
    if !table_exists(db, "person_role_progression")? {
        return Ok(None);
    }

    db.conn_ref()
        .query_row(
            "SELECT started_at, ended_at, title, org, seniority, source_refs_json
             FROM person_role_progression
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND datetime(started_at) <= datetime(?3)
               AND (ended_at IS NULL OR datetime(ended_at) > datetime(?3))
               AND source_invalidated_at IS NULL
             ORDER BY datetime(started_at) DESC, started_at DESC
             LIMIT 1",
            params![entity_type, entity_id, observed_at.to_rfc3339()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("query active person_role_progression row: {error}"))?
        .map(stored_role_entry_from_row)
        .transpose()
}

fn next_role_started_at(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    observed_at: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, String> {
    if !table_exists(db, "person_role_progression")? {
        return Ok(None);
    }

    db.conn_ref()
        .query_row(
            "SELECT started_at
             FROM person_role_progression
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND datetime(started_at) > datetime(?3)
               AND source_invalidated_at IS NULL
             ORDER BY datetime(started_at) ASC, started_at ASC
             LIMIT 1",
            params![entity_type, entity_id, observed_at.to_rfc3339()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("query next person_role_progression row: {error}"))?
        .as_deref()
        .map(parse_db_datetime)
        .transpose()
}

fn stored_role_entry_from_row(
    (started_at_raw, ended_at_raw, title, org, seniority, source_refs_json): (
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        String,
    ),
) -> Result<StoredRoleEntry, String> {
    let started_at = parse_db_datetime(&started_at_raw)?;
    let ended_at = ended_at_raw.as_deref().map(parse_db_datetime).transpose()?;
    Ok(StoredRoleEntry {
        started_at,
        ended_at,
        title,
        org,
        seniority,
        source_refs_json,
    })
}

fn count_meetings(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<u32, String> {
    if !table_exists(db, "meetings")? || !table_exists(db, "meeting_entities")? {
        return Ok(0);
    }

    db.conn_ref()
        .query_row(
            "SELECT COUNT(DISTINCT m.id)
             FROM meetings m
             JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_type = ?1
               AND me.entity_id = ?2
               AND datetime(m.start_time) >= datetime(?3)
               AND datetime(m.start_time) < datetime(?4)",
            params![entity_type, entity_id, start.to_rfc3339(), end.to_rfc3339()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("count meetings for engagement: {error}"))
        .and_then(u32_from_i64)
}

struct EngagementCurveRow {
    week_start: DateTime<Utc>,
    meetings_count: u32,
    emails_count: u32,
    bidirectional_ratio: f32,
    source_refs_json: String,
}

struct EngagementRefreshPlan {
    rows: Vec<EngagementCurveRow>,
    last_completed_week_start: Option<DateTime<Utc>>,
}

fn engagement_rows_to_refresh(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    latest_week_start: DateTime<Utc>,
    retention_cutoff: DateTime<Utc>,
) -> Result<EngagementRefreshPlan, String> {
    let mut rows = Vec::new();
    rows.push(engagement_row_for_week(
        db,
        entity_type,
        entity_id,
        latest_week_start,
    )?);

    let mut last_completed_week_start = None;
    let Some(mut week_start) = next_backfill_week_start(
        db,
        entity_type,
        entity_id,
        latest_week_start,
        retention_cutoff,
    )?
    else {
        return Ok(EngagementRefreshPlan {
            rows,
            last_completed_week_start,
        });
    };

    for _ in 0..ENGAGEMENT_BACKFILL_CHUNK_WEEKS {
        if week_start < retention_cutoff {
            break;
        }
        let row = engagement_row_for_week(db, entity_type, entity_id, week_start)?;
        if row.meetings_count > 0 || row.emails_count > 0 {
            rows.push(row);
        }
        last_completed_week_start = Some(week_start);
        week_start -= Duration::seconds(SECONDS_PER_WEEK);
    }

    Ok(EngagementRefreshPlan {
        rows,
        last_completed_week_start,
    })
}

fn next_backfill_week_start(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    latest_week_start: DateTime<Utc>,
    retention_cutoff: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, String> {
    if !table_exists(db, "temporal_backfill_state")? {
        return Ok(Some(
            latest_week_start - Duration::seconds(SECONDS_PER_WEEK),
        ));
    }

    let last_completed = db
        .conn_ref()
        .query_row(
            "SELECT last_completed_week_start
             FROM temporal_backfill_state
             WHERE entity_type = ?1 AND entity_id = ?2 AND ability_id = ?3",
            params![entity_type, entity_id, REFRESH_ENGAGEMENT_CURVE_ABILITY_ID],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("query temporal_backfill_state: {error}"))?
        .as_deref()
        .map(parse_db_datetime)
        .transpose()?;

    let Some(last_completed) = last_completed else {
        return Ok(Some(
            latest_week_start - Duration::seconds(SECONDS_PER_WEEK),
        ));
    };

    if last_completed <= retention_cutoff {
        Ok(None)
    } else {
        Ok(Some(last_completed - Duration::seconds(SECONDS_PER_WEEK)))
    }
}

fn engagement_row_for_week(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    week_start: DateTime<Utc>,
) -> Result<EngagementCurveRow, String> {
    let week_end = week_start + Duration::seconds(SECONDS_PER_WEEK);
    let meetings_count = count_meetings(db, entity_type, entity_id, week_start, week_end)?;
    let email_stats = count_emails(db, entity_type, entity_id, week_start, week_end)?;
    let bidirectional_ratio =
        bidirectional_ratio(email_stats.user_last_count, email_stats.other_last_count);
    let source_refs_json =
        engagement_source_refs_json(db, entity_type, entity_id, week_start, week_end)?;

    Ok(EngagementCurveRow {
        week_start,
        meetings_count,
        emails_count: email_stats.total_count,
        bidirectional_ratio,
        source_refs_json,
    })
}

struct EmailStats {
    total_count: u32,
    user_last_count: u32,
    other_last_count: u32,
}

fn count_emails(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<EmailStats, String> {
    if !table_exists(db, "emails")? {
        return Ok(EmailStats {
            total_count: 0,
            user_last_count: 0,
            other_last_count: 0,
        });
    }

    let (total, user_last, other_last) = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(DISTINCT email_id),
                    SUM(CASE WHEN COALESCE(user_is_last_sender, 0) = 1 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN COALESCE(user_is_last_sender, 0) = 0 THEN 1 ELSE 0 END)
             FROM emails
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND received_at IS NOT NULL
               AND datetime(received_at) >= datetime(?3)
               AND datetime(received_at) < datetime(?4)",
            params![entity_type, entity_id, start.to_rfc3339(), end.to_rfc3339()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                ))
            },
        )
        .map_err(|error| format!("count emails for engagement: {error}"))?;

    Ok(EmailStats {
        total_count: u32_from_i64(total)?,
        user_last_count: u32_from_i64(user_last)?,
        other_last_count: u32_from_i64(other_last)?,
    })
}

fn engagement_source_refs_json(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String, String> {
    let mut refs = Vec::new();

    if table_exists(db, "meetings")? && table_exists(db, "meeting_entities")? {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT DISTINCT m.id, m.start_time
                 FROM meetings m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_type = ?1
                   AND me.entity_id = ?2
                   AND datetime(m.start_time) >= datetime(?3)
                   AND datetime(m.start_time) < datetime(?4)
                 ORDER BY datetime(m.start_time) ASC, m.id ASC",
            )
            .map_err(|error| format!("prepare meeting engagement sources: {error}"))?;
        let rows = stmt
            .query_map(
                params![entity_type, entity_id, start.to_rfc3339(), end.to_rfc3339()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|error| format!("query meeting engagement sources: {error}"))?;
        for row in rows {
            let (meeting_id, observed_at_raw) =
                row.map_err(|error| format!("map meeting engagement source: {error}"))?;
            let observed_at = parse_db_datetime(&observed_at_raw)?;
            refs.push(SourceRef::Direct {
                data_source: DataSource::Google,
                identifier: SourceIdentifier::Meeting {
                    meeting_id: MeetingId::new(meeting_id),
                },
                observed_at,
                source_asof: Some(observed_at),
            });
        }
    }

    if table_exists(db, "emails")? {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT DISTINCT email_id, received_at
                 FROM emails
                 WHERE entity_type = ?1
                   AND entity_id = ?2
                   AND received_at IS NOT NULL
                   AND datetime(received_at) >= datetime(?3)
                   AND datetime(received_at) < datetime(?4)
                 ORDER BY datetime(received_at) ASC, email_id ASC",
            )
            .map_err(|error| format!("prepare email engagement sources: {error}"))?;
        let rows = stmt
            .query_map(
                params![entity_type, entity_id, start.to_rfc3339(), end.to_rfc3339()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|error| format!("query email engagement sources: {error}"))?;
        for row in rows {
            let (email_id, observed_at_raw) =
                row.map_err(|error| format!("map email engagement source: {error}"))?;
            let observed_at = parse_db_datetime(&observed_at_raw)?;
            refs.push(SourceRef::Direct {
                data_source: DataSource::Google,
                identifier: SourceIdentifier::EmailMessage {
                    email_id: EmailId::new(email_id),
                    message_id: None,
                },
                observed_at,
                source_asof: Some(observed_at),
            });
        }
    }

    serde_json::to_string(&refs)
        .map_err(|error| format!("serialize engagement source refs: {error}"))
}

pub fn mark_source_invalidated_in_db(
    db: &ActionDb,
    source: &str,
    invalidated_at: DateTime<Utc>,
) -> Result<usize, String> {
    let mut marked = 0usize;
    marked += mark_temporal_table_source_invalidated(
        db,
        "entity_engagement_curve",
        "week_start",
        source,
        invalidated_at,
    )?;
    marked += mark_temporal_table_source_invalidated(
        db,
        "person_role_progression",
        "started_at",
        source,
        invalidated_at,
    )?;
    Ok(marked)
}

fn mark_temporal_table_source_invalidated(
    db: &ActionDb,
    table_name: &'static str,
    key_column: &'static str,
    source: &str,
    invalidated_at: DateTime<Utc>,
) -> Result<usize, String> {
    if !table_exists(db, table_name)? || !column_exists(db, table_name, "source_invalidated_at")? {
        return Ok(0);
    }

    let select_sql = format!(
        "SELECT entity_type, entity_id, {key_column}, source_refs_json
         FROM {table_name}
         WHERE source_invalidated_at IS NULL"
    );
    let mut stmt = db
        .conn_ref()
        .prepare(&select_sql)
        .map_err(|error| format!("prepare {table_name} source invalidation scan: {error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| format!("query {table_name} source invalidation scan: {error}"))?;

    let mut keys = Vec::new();
    for row in rows {
        let (entity_type, entity_id, key, source_refs_json) =
            row.map_err(|error| format!("map {table_name} source invalidation row: {error}"))?;
        let source_refs = parse_source_refs(&source_refs_json);
        if source_refs
            .iter()
            .any(|source_ref| source_ref_matches_revoked_source(source_ref, source))
        {
            keys.push((entity_type, entity_id, key));
        }
    }

    let update_sql = format!(
        "UPDATE {table_name}
         SET source_invalidated_at = ?4
         WHERE entity_type = ?1
           AND entity_id = ?2
           AND {key_column} = ?3
           AND source_invalidated_at IS NULL"
    );
    let mut updated = 0usize;
    for (entity_type, entity_id, key) in keys {
        updated += db
            .conn_ref()
            .execute(
                &update_sql,
                params![entity_type, entity_id, key, invalidated_at.to_rfc3339()],
            )
            .map_err(|error| format!("mark {table_name} source invalidated: {error}"))?;
    }

    Ok(updated)
}

fn source_ref_matches_revoked_source(source_ref: &SourceRef, source: &str) -> bool {
    match source_ref {
        SourceRef::Direct { data_source, .. } => {
            data_source_matches_revoked_source(data_source, source)
        }
        SourceRef::Source { .. } | SourceRef::Child { .. } => false,
    }
}

fn data_source_matches_revoked_source(data_source: &DataSource, source: &str) -> bool {
    let source = normalize_source_name(source);
    match data_source {
        DataSource::User => source == "user",
        DataSource::Clay => source == "clay",
        DataSource::Google => source == "google",
        DataSource::Ai => source == "ai",
        DataSource::CoAttendance => source == "co_attendance" || source == "co-attendance",
        DataSource::LocalEnrichment => source == "local_enrichment",
        DataSource::Glean { .. } => source == "glean" || source.starts_with("glean_"),
        DataSource::Other(name) => {
            let name = normalize_source_name(name.as_str());
            name == source || (source == "glean" && (name == "glean" || name.starts_with("glean_")))
        }
        _ => false,
    }
}

fn normalize_source_name(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

fn bidirectional_ratio(user_last_count: u32, other_last_count: u32) -> f32 {
    let total = user_last_count + other_last_count;
    if total == 0 || user_last_count == 0 || other_last_count == 0 {
        return 0.0;
    }

    let balanced = user_last_count.min(other_last_count) * 2;
    balanced as f32 / total as f32
}

fn complete_week_start(as_of: DateTime<Utc>) -> DateTime<Utc> {
    let target_date = as_of.date_naive() - Duration::days(7);
    let days_from_monday = i64::from(target_date.weekday().num_days_from_monday());
    let week_start = target_date - Duration::days(days_from_monday);
    Utc.from_utc_datetime(
        &week_start
            .and_hms_opt(0, 0, 0)
            .expect("midnight is always valid"),
    )
}

fn table_exists(db: &ActionDb, table_name: &str) -> Result<bool, String> {
    db.conn_ref()
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = ?1
            )",
            params![table_name],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|error| format!("check table {table_name}: {error}"))
}

fn column_exists(db: &ActionDb, table_name: &str, column_name: &str) -> Result<bool, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|error| format!("inspect columns for {table_name}: {error}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("query columns for {table_name}: {error}"))?;

    for row in rows {
        if row.map_err(|error| format!("read column for {table_name}: {error}"))? == column_name {
            return Ok(true);
        }
    }

    Ok(false)
}

fn parse_source_refs(value: &str) -> Vec<SourceRef> {
    serde_json::from_str(value).unwrap_or_default()
}

fn parse_db_datetime(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|parsed| Utc.from_utc_datetime(&parsed))
        })
        .map_err(|error| format!("parse datetime `{value}`: {error}"))
}

fn normalize_required(value: String, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{field_name} must be non-empty"))
    } else {
        Ok(trimmed.to_string())
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn i64_from_usize(value: usize) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("usize value {value} does not fit i64"))
}

fn u16_from_i64(value: i64) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("i64 value {value} does not fit u16"))
}

fn u32_from_i64(value: i64) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("i64 value {value} does not fit u32"))
}
