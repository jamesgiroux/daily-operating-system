//! App-facing re-export of the ability runtime `ServiceContext` surface.
//!
//! The `abilities-runtime` crate owns the public context/capability types that
//! ability code can compile against. This module adds DailyOS-only live reader
//! adapters that reach SQLite from the app crate, keeping those raw handles out
//! of the ability runtime dependency graph.

use std::sync::Arc;

pub use abilities_runtime::services::context::*;

use crate::abilities::temporal::{
    DetectRoleChangeInput, DetectRoleChangeResult, RefreshEngagementCurveInput,
    RefreshEngagementCurveResult, TemporalMaintenanceFuture, TemporalMaintenanceHandle,
    TrajectoryQueryDepth, TrajectoryReadFuture, TrajectoryReadHandle,
};

pub struct LiveEntityContextReader;
pub struct LiveEntityContextClaimReader;
pub struct LivePrepareMeetingContextReader;
pub struct LiveTemporalWorkspaceReader;

pub fn attach_live_workspace_readers(ctx: ServiceContext<'_>) -> ServiceContext<'_> {
    ctx.with_entity_context_reader(Arc::new(LiveEntityContextReader))
        .with_entity_context_claim_reader(Arc::new(LiveEntityContextClaimReader))
        .with_prepare_meeting_context_reader(Arc::new(LivePrepareMeetingContextReader))
        .with_trajectory_reader(Arc::new(LiveTemporalWorkspaceReader))
        .with_temporal_maintenance(Arc::new(LiveTemporalWorkspaceReader))
}

impl EntityContextReadHandle for LiveEntityContextReader {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                read_entity_context_entries_from_db(&db, &entity_type, &entity_id)
            })
            .await
            .map_err(|error| format!("Entity context read task failed: {error}"))?
        })
    }
}

impl EntityContextClaimReadHandle for LiveEntityContextClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::claims::load_entity_context_claims_active(
                    &db,
                    &entity_type,
                    &entity_id,
                    depth,
                )
                .map_err(|error| format!("Entity context claim read failed: {error}"))
            })
            .await
            .map_err(|error| format!("Entity context claim read task failed: {error}"))?
        })
    }
}

impl PrepareMeetingContextReadHandle for LivePrepareMeetingContextReader {
    fn read_prepare_meeting_context<'a>(
        &'a self,
        meeting_id: String,
    ) -> PrepareMeetingContextReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::meetings::load_prepare_meeting_context_snapshot(&db, &meeting_id)
            })
            .await
            .map_err(|error| format!("prepare_meeting context read task failed: {error}"))?
        })
    }
}

impl TrajectoryReadHandle for LiveTemporalWorkspaceReader {
    fn read_trajectory_bundle<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: TrajectoryQueryDepth,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TrajectoryReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = crate::db::ActionDb::open()
                    .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::temporal::read_trajectory_bundle_from_db(
                    &db,
                    &entity_type,
                    &entity_id,
                    depth,
                    computed_at,
                )
            })
            .await
            .map_err(|error| format!("trajectory read task failed: {error}"))?
        })
    }
}

impl TemporalMaintenanceHandle for LiveTemporalWorkspaceReader {
    fn refresh_engagement_curve<'a>(
        &'a self,
        input: RefreshEngagementCurveInput,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TemporalMaintenanceFuture<'a, RefreshEngagementCurveResult> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = crate::db::ActionDb::open()
                    .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::temporal::refresh_engagement_curve_in_db(&db, input, computed_at)
            })
            .await
            .map_err(|error| format!("refresh_engagement_curve task failed: {error}"))?
        })
    }

    fn detect_role_change<'a>(
        &'a self,
        input: DetectRoleChangeInput,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TemporalMaintenanceFuture<'a, DetectRoleChangeResult> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = crate::db::ActionDb::open()
                    .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::temporal::detect_role_change_in_db(&db, input, computed_at)
            })
            .await
            .map_err(|error| format!("detect_role_change task failed: {error}"))?
        })
    }
}

impl EntityContextReadHandle for crate::db_service::PooledConnection {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a> {
        let reader = self.clone();
        Box::pin(async move {
            let entries = reader
                .call(move |conn| {
                    let db = crate::db::ActionDb::from_conn(conn);
                    read_entity_context_entries_from_db(db, &entity_type, &entity_id)
                        .map_err(rusqlite::Error::InvalidParameterName)
                })
                .await
                .map_err(|error| format!("DB read error: {error}"))?;
            Ok(entries)
        })
    }
}

pub(crate) fn read_entity_context_entries_from_db(
    db: &crate::db::ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<crate::types::EntityContextEntry>, String> {
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT id, entity_type, entity_id, title, content, created_at, updated_at
             FROM entity_context_entries
             WHERE entity_type = ?1 AND entity_id = ?2
             ORDER BY created_at DESC",
        )
        .map_err(|error| format!("Failed to prepare entity context query: {error}"))?;

    let entries = stmt
        .query_map(rusqlite::params![entity_type, entity_id], |row| {
            Ok(crate::types::EntityContextEntry {
                id: row.get("id")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                title: row.get::<_, String>("title")?.into(),
                content: row.get::<_, String>("content")?.into(),
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|error| format!("Failed to query entity context entries: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to map entity context entries: {error}"))?;

    Ok(entries)
}
