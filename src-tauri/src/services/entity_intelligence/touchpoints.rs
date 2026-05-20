//! DOS-460 — canonical entity touchpoints reader.
//!
//! Per L0 packet `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.2.
//!
//! Reads upcoming + recent meeting-shaped touchpoints for a subject, with
//! subject-scope expansion for parent/child accounts and attendee-match
//! fallback for person subjects. Each returned snapshot carries an explicit
//! `inclusion_reason` so the envelope composer can render the "why" verbatim
//! without recomputing the filter.
//!
//! The reader returns a `Snapshot` shape; the projection to
//! `TouchpointBundle` happens in the `get_entity_intelligence` producer
//! (DOS-459). This keeps the substrate read seam narrow (no envelope DTO
//! pollution) and lets the daily-briefing readiness path consume the same
//! `EntityTouchpointsSnapshot::filter_description` as the candidate-set
//! primitive (AC-460.7).

use std::collections::BTreeSet;

use abilities_runtime::services::context::{
    EntityTouchpointSnapshot, EntityTouchpointsQuery, EntityTouchpointsReadError,
    EntityTouchpointsReadFuture, EntityTouchpointsReadHandle, EntityTouchpointsSnapshot,
    TouchpointInclusionReason,
};
use rusqlite::params;

use crate::db::ActionDb;

/// Live SQLite-backed implementation of [`EntityTouchpointsReadHandle`].
///
/// Attached by `attach_live_workspace_readers` so the `get_entity_intelligence`
/// producer can populate the `TouchpointBundle` from real meeting data.
pub struct LiveEntityTouchpointsReader;

impl EntityTouchpointsReadHandle for LiveEntityTouchpointsReader {
    fn read_entity_touchpoints<'a>(
        &'a self,
        query: EntityTouchpointsQuery,
    ) -> EntityTouchpointsReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                    .map_err(|error| {
                        EntityTouchpointsReadError::ReadFailed(format!(
                            "Database unavailable: {error}"
                        ))
                    })?;
                read_entity_touchpoints_from_db(&db, &query)
            })
            .await
            .map_err(|error| {
                EntityTouchpointsReadError::ReadFailed(format!(
                    "Entity touchpoints read task failed: {error}"
                ))
            })?
        })
    }
}

/// Pure-SQL composer — public for fixture-driven tests that want to skip the
/// async spawn-blocking wrapper.
pub fn read_entity_touchpoints_from_db(
    db: &ActionDb,
    query: &EntityTouchpointsQuery,
) -> Result<EntityTouchpointsSnapshot, EntityTouchpointsReadError> {
    let entity_type = query.entity_type.as_str();
    let entity_id = query.entity_id.as_str();

    // Subject-scope expansion. For accounts: include parent/child account IDs
    // so a parent account's bundle surfaces child-account touchpoints (and
    // vice versa). For projects: include parent project. For persons: scope
    // is just the person id — multi-account-person bleed prevention happens
    // in the join filter below.
    let (also_includes, scope_ids) = expand_subject_scope(db, entity_type, entity_id)
        .map_err(EntityTouchpointsReadError::ReadFailed)?;

    let upcoming = read_meetings_for_scope(
        db,
        &ReadMeetingsArgs {
            entity_type,
            scope_ids: &scope_ids,
            primary_id: entity_id,
            now: &query.now,
            window_days: query.upcoming_window_days,
            cap: query.per_side_cap,
            direction: TimeDirection::Upcoming,
        },
    )
    .map_err(EntityTouchpointsReadError::ReadFailed)?;

    let recent = read_meetings_for_scope(
        db,
        &ReadMeetingsArgs {
            entity_type,
            scope_ids: &scope_ids,
            primary_id: entity_id,
            now: &query.now,
            window_days: query.recent_window_days,
            cap: query.per_side_cap,
            direction: TimeDirection::Recent,
        },
    )
    .map_err(EntityTouchpointsReadError::ReadFailed)?;

    let filter_description = describe_filter(entity_type, entity_id, query, &also_includes);

    Ok(EntityTouchpointsSnapshot {
        subject_entity_type: entity_type.to_string(),
        subject_entity_id: entity_id.to_string(),
        upcoming,
        recent,
        also_includes,
        filter_description,
    })
}

#[derive(Debug, Clone, Copy)]
enum TimeDirection {
    Upcoming,
    Recent,
}

struct ReadMeetingsArgs<'a> {
    entity_type: &'a str,
    scope_ids: &'a [String],
    primary_id: &'a str,
    now: &'a chrono::DateTime<chrono::Utc>,
    window_days: u16,
    cap: usize,
    direction: TimeDirection,
}

fn read_meetings_for_scope(
    db: &ActionDb,
    args: &ReadMeetingsArgs<'_>,
) -> Result<Vec<EntityTouchpointSnapshot>, String> {
    if args.scope_ids.is_empty() {
        return Ok(Vec::new());
    }

    let now_iso = args.now.to_rfc3339();
    let bound_iso = match args.direction {
        TimeDirection::Upcoming => {
            (*args.now + chrono::Duration::days(i64::from(args.window_days))).to_rfc3339()
        }
        TimeDirection::Recent => {
            (*args.now - chrono::Duration::days(i64::from(args.window_days))).to_rfc3339()
        }
    };

    let placeholders: Vec<String> = (0..args.scope_ids.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
    let placeholders_csv = placeholders.join(", ");

    let (range_clause, order_clause) = match args.direction {
        TimeDirection::Upcoming => (
            "m.start_time >= ?1 AND m.start_time <= ?2",
            "ORDER BY m.start_time ASC",
        ),
        TimeDirection::Recent => (
            "m.start_time <= ?1 AND m.start_time >= ?2",
            "ORDER BY m.start_time DESC",
        ),
    };

    let sql = if args.entity_type == "person" {
        let second_placeholders: Vec<String> = (0..args.scope_ids.len())
            .map(|i| format!("?{}", i + 3 + args.scope_ids.len()))
            .collect();
        let second_csv = second_placeholders.join(", ");
        format!(
            "SELECT DISTINCT
                m.id,
                m.title,
                m.meeting_type,
                m.start_time,
                m.end_time,
                CASE WHEN me.entity_id IS NOT NULL THEN 'subject_match' ELSE 'attendee_match' END AS reason,
                COALESCE(me.entity_id, ma.person_id) AS matched_id
             FROM meetings m
             LEFT JOIN meeting_entities me
                 ON me.meeting_id = m.id
                AND me.entity_type = 'person'
                AND me.entity_id IN ({first_csv})
             LEFT JOIN meeting_attendees ma
                 ON ma.meeting_id = m.id
                AND ma.person_id IN ({second_csv})
             WHERE (me.entity_id IS NOT NULL OR ma.person_id IS NOT NULL)
               AND {range_clause}
             {order_clause}
             LIMIT ?{cap_index}",
            first_csv = placeholders_csv,
            second_csv = second_csv,
            range_clause = range_clause,
            order_clause = order_clause,
            cap_index = args.scope_ids.len() * 2 + 3,
        )
    } else {
        format!(
            "SELECT DISTINCT
                m.id,
                m.title,
                m.meeting_type,
                m.start_time,
                m.end_time,
                'subject_match' AS reason,
                me.entity_id AS matched_id
             FROM meetings m
             INNER JOIN meeting_entities me
                 ON me.meeting_id = m.id
                AND me.entity_type = ?{type_index}
                AND me.entity_id IN ({placeholders_csv})
             WHERE {range_clause}
             {order_clause}
             LIMIT ?{cap_index}",
            placeholders_csv = placeholders_csv,
            range_clause = range_clause,
            order_clause = order_clause,
            type_index = args.scope_ids.len() + 3,
            cap_index = args.scope_ids.len() + 4,
        )
    };

    let conn = db.conn_ref();
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let cap_capped = args.cap.min(500) as i64;
    let mut bound_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    bound_params.push(Box::new(now_iso));
    bound_params.push(Box::new(bound_iso));
    for id in args.scope_ids {
        bound_params.push(Box::new(id.clone()));
    }
    if args.entity_type == "person" {
        for id in args.scope_ids {
            bound_params.push(Box::new(id.clone()));
        }
        bound_params.push(Box::new(cap_capped));
    } else {
        bound_params.push(Box::new(args.entity_type.to_string()));
        bound_params.push(Box::new(cap_capped));
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = bound_params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query_map(rusqlite::params_from_iter(param_refs), |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let kind: String = row.get(2)?;
            let start: Option<String> = row.get(3)?;
            let end: Option<String> = row.get(4)?;
            let reason: String = row.get(5)?;
            let matched_id: String = row.get(6)?;
            Ok((id, title, kind, start, end, reason, matched_id))
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        let (meeting_id, title, kind, starts_at, ends_at, reason_str, matched_id) =
            row.map_err(|e| e.to_string())?;
        let inclusion_reason =
            classify_inclusion(args.entity_type, args.primary_id, &matched_id, &reason_str);
        out.push(EntityTouchpointSnapshot {
            meeting_id,
            title,
            kind,
            starts_at: starts_at.clone(),
            ends_at,
            subject_entity_type: args.entity_type.to_string(),
            subject_entity_id: matched_id,
            inclusion_reason,
            exclusion_reason: None,
            source_asof: starts_at,
        });
    }
    Ok(out)
}

fn classify_inclusion(
    entity_type: &str,
    primary_id: &str,
    matched_id: &str,
    reason_str: &str,
) -> TouchpointInclusionReason {
    match (entity_type, reason_str) {
        ("person", "attendee_match") => TouchpointInclusionReason::AttendeeMatch,
        (_, "subject_match") if matched_id == primary_id => {
            TouchpointInclusionReason::SubjectMatch
        }
        // Matched via expanded scope (parent/child account, parent project) —
        // this is a typed entity link, not a raw subject match.
        (_, "subject_match") => TouchpointInclusionReason::EntityLink,
        // Defensive fallback — never panic on an unknown reason string.
        _ => TouchpointInclusionReason::SubjectMatch,
    }
}

/// Expand the subject scope to include parent/child accounts (or parent
/// project). Returns `(also_includes, scope_ids)` where `scope_ids` always
/// contains the primary id at index 0.
fn expand_subject_scope(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<(Vec<(String, String)>, Vec<String>), String> {
    let mut scope = BTreeSet::new();
    scope.insert(entity_id.to_string());
    let mut also = Vec::new();

    match entity_type {
        "account" => {
            if let Some(parent_id) = read_optional_string(
                db,
                "SELECT parent_id FROM accounts WHERE id = ?1",
                params![entity_id],
            )? {
                if !parent_id.is_empty() && scope.insert(parent_id.clone()) {
                    also.push(("account".to_string(), parent_id));
                }
            }
            let children = read_string_list(
                db,
                "SELECT id FROM accounts WHERE parent_id = ?1",
                params![entity_id],
            )?;
            for child in children {
                if scope.insert(child.clone()) {
                    also.push(("account".to_string(), child));
                }
            }
        }
        "project" => {
            if let Some(parent_id) = read_optional_string(
                db,
                "SELECT parent_id FROM projects WHERE id = ?1",
                params![entity_id],
            )? {
                if !parent_id.is_empty() && scope.insert(parent_id.clone()) {
                    also.push(("project".to_string(), parent_id));
                }
            }
        }
        // "person" — no expansion. Multi-account-person bleed prevention is
        // handled at the join: person subjects do NOT pull in their primary
        // account's meetings unless the person was an attendee.
        _ => {}
    }

    let scope_ids: Vec<String> = scope.into_iter().collect();
    Ok((also, scope_ids))
}

fn read_optional_string(
    db: &ActionDb,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> Result<Option<String>, String> {
    let conn = db.conn_ref();
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params).map_err(|e| e.to_string())?;
    match rows.next().map_err(|e| e.to_string())? {
        Some(row) => Ok(row.get::<_, Option<String>>(0).ok().flatten()),
        None => Ok(None),
    }
}

fn read_string_list(
    db: &ActionDb,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> Result<Vec<String>, String> {
    let conn = db.conn_ref();
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params, |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn describe_filter(
    entity_type: &str,
    entity_id: &str,
    query: &EntityTouchpointsQuery,
    also: &[(String, String)],
) -> String {
    let mut parts = vec![format!(
        "{entity_type}:{entity_id} touchpoints within -{recent}d..+{upcoming}d window",
        recent = query.recent_window_days,
        upcoming = query.upcoming_window_days,
    )];
    if !also.is_empty() {
        parts.push(format!("scope_includes={}", also.len()));
    }
    if entity_type == "person" {
        parts.push("attendee_match_fallback=on".to_string());
    }
    parts.join("; ")
}

// ---- tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    //! Subject-isolation tests use the abilities-runtime DTOs directly.
    //! Full DB-backed coverage lives in the DOS-461 harness
    //! (`src-tauri/tests/entity_intelligence_no_bypass/`).

    use super::*;

    #[test]
    fn classify_inclusion_distinguishes_subject_match_vs_entity_link() {
        assert_eq!(
            classify_inclusion("account", "acc-parent", "acc-parent", "subject_match"),
            TouchpointInclusionReason::SubjectMatch
        );
        // Matched via parent/child scope expansion.
        assert_eq!(
            classify_inclusion("account", "acc-parent", "acc-child", "subject_match"),
            TouchpointInclusionReason::EntityLink
        );
        assert_eq!(
            classify_inclusion("person", "p-1", "p-1", "attendee_match"),
            TouchpointInclusionReason::AttendeeMatch
        );
        assert_eq!(
            classify_inclusion("person", "p-1", "p-1", "subject_match"),
            TouchpointInclusionReason::SubjectMatch
        );
    }

    #[test]
    fn classify_inclusion_unknown_reason_falls_back_safely() {
        // Defensive fallback — never panic.
        assert_eq!(
            classify_inclusion("project", "proj-1", "proj-1", "wat"),
            TouchpointInclusionReason::SubjectMatch
        );
    }

    #[test]
    fn describe_filter_mentions_window_and_attendee_fallback() {
        let q = EntityTouchpointsQuery {
            entity_type: "person".to_string(),
            entity_id: "p-1".to_string(),
            now: chrono::Utc::now(),
            upcoming_window_days: 14,
            recent_window_days: 30,
            per_side_cap: 50,
        };
        let desc = describe_filter("person", "p-1", &q, &[]);
        assert!(desc.contains("person:p-1"));
        assert!(desc.contains("-30d..+14d"));
        assert!(desc.contains("attendee_match_fallback=on"));
    }

    #[test]
    fn describe_filter_mentions_scope_expansion_for_accounts() {
        let q = EntityTouchpointsQuery {
            entity_type: "account".to_string(),
            entity_id: "acc-parent".to_string(),
            now: chrono::Utc::now(),
            upcoming_window_days: 14,
            recent_window_days: 30,
            per_side_cap: 50,
        };
        let also = vec![
            ("account".to_string(), "acc-child-1".to_string()),
            ("account".to_string(), "acc-child-2".to_string()),
        ];
        let desc = describe_filter("account", "acc-parent", &q, &also);
        assert!(desc.contains("scope_includes=2"));
        // Account subjects never get the attendee fallback.
        assert!(!desc.contains("attendee_match_fallback"));
    }

    #[test]
    fn describe_filter_account_no_scope_includes_no_attendee_fallback() {
        let q = EntityTouchpointsQuery {
            entity_type: "account".to_string(),
            entity_id: "acc-1".to_string(),
            now: chrono::Utc::now(),
            upcoming_window_days: 14,
            recent_window_days: 30,
            per_side_cap: 50,
        };
        let desc = describe_filter("account", "acc-1", &q, &[]);
        assert!(!desc.contains("scope_includes"));
        assert!(!desc.contains("attendee_match_fallback"));
    }
}
