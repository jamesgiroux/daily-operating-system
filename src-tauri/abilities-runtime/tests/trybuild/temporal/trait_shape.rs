use chrono::{TimeZone, Utc};
use abilities_runtime::abilities::provenance::{EntityId, SourceIndex, SourceRef};
use abilities_runtime::abilities::temporal::{
    DataPoint, DetectRoleChangeInput, DetectRoleChangeResult, EngagementWindow,
    RefreshEngagementCurveInput, RefreshEngagementCurveResult, RoleEntry,
    TemporalMaintenanceFuture, TemporalMaintenanceHandle, TrajectoryBundle, TrajectoryKind,
    TrajectoryQueryDepth, TrajectoryReadFuture, TrajectoryReadHandle, TrajectorySnapshot,
};

struct FixtureTemporal;

impl TrajectoryReadHandle for FixtureTemporal {
    fn read_trajectory_bundle<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        _depth: TrajectoryQueryDepth,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TrajectoryReadFuture<'a> {
        Box::pin(async move {
            let point = DataPoint {
                at: computed_at,
                value: EngagementWindow::new(1, 2, 1.0).unwrap(),
                source_refs: vec![SourceRef::Source {
                    source_index: SourceIndex(0),
                }],
            };
            let snapshot = TrajectorySnapshot::new(
                TrajectoryKind::EngagementCurve,
                entity_type,
                EntityId::new(entity_id),
                vec![point],
                computed_at,
                1.0,
            )
            .unwrap();
            Ok(TrajectoryBundle {
                engagement_curve: Some(snapshot),
                role_progression: None,
            })
        })
    }
}

impl TemporalMaintenanceHandle for FixtureTemporal {
    fn refresh_engagement_curve<'a>(
        &'a self,
        input: RefreshEngagementCurveInput,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TemporalMaintenanceFuture<'a, RefreshEngagementCurveResult> {
        Box::pin(async move {
            Ok(RefreshEngagementCurveResult {
                entity_type: input.entity_type,
                entity_id: input.entity_id,
                week_start: computed_at,
                rows_written: 1,
                retained_weeks: 1,
                computed_at,
            })
        })
    }

    fn detect_role_change<'a>(
        &'a self,
        input: DetectRoleChangeInput,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TemporalMaintenanceFuture<'a, DetectRoleChangeResult> {
        Box::pin(async move {
            let started_at = input.observed_at.unwrap_or(computed_at);
            let _entry = RoleEntry {
                started_at,
                ended_at: None,
                title: input.title,
                org: input.org,
                seniority: input.seniority,
            };
            Ok(DetectRoleChangeResult {
                entity_type: input.entity_type,
                entity_id: input.entity_id,
                appended: true,
                current_started_at: started_at,
                prior_ended_at: None,
                computed_at,
            })
        })
    }
}

fn assert_traits<T: TrajectoryReadHandle + TemporalMaintenanceHandle>() {}

fn main() {
    assert_traits::<FixtureTemporal>();
    let depth = TrajectoryQueryDepth::Weeks(52);
    assert_eq!(depth.limit(), 52);
    let _ = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
}
