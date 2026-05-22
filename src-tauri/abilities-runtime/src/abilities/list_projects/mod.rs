//! `list_projects` Read ability — paginated project index for the
//! v1.4.4 W2 Projects list shell (`wp/dailyos/blocks/projects-index`).
//! Mirror of `list_accounts`; see that module's docs for cursor / actor
//! discipline (W1 substrate extension; "every surface in 1.4.4. No
//! deferrals.").

pub mod contracts;
pub mod producer;

pub use contracts::{ProjectListFilter, ProjectListInput, ProjectSummary, ProjectTrajectory};

use dailyos_abilities_macro::ability;

use crate::abilities::get_entity_intelligence::contracts::Paginated;
use crate::abilities::{AbilityContext, AbilityResult};

pub(crate) const ABILITY_NAME: &str = "list_projects";
pub(crate) const ABILITY_SCHEMA_VERSION: u32 = 1;

#[ability(
    name = "list_projects",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, Agent, SurfaceClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.projects_list"],
    mcp_exposure = None,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn list_projects(
    ctx: &AbilityContext<'_>,
    input: ProjectListInput,
) -> AbilityResult<Paginated<ProjectSummary>> {
    producer::list_projects(ctx, input).await
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::*;
    use crate::abilities::list_pagination::decode_cursor;
    use crate::abilities::registry::{AbilityRegistry, McpClientId};
    use crate::abilities::{AbilityErrorKind, Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
        ProviderError, ProviderKind,
    };
    use crate::sensitivity::ClaimDismissalSurface;
    use crate::services::context::{
        FixedClock, ProjectListQuery, ProjectListReadError, ProjectListReadFuture,
        ProjectListReadHandle, ProjectListSnapshot, ProjectListSummary, SeedableRng,
        ServiceContext,
    };

    struct FixtureReader {
        rows: Mutex<Vec<ProjectListSummary>>,
        calls: AtomicUsize,
        data_shifted_after: Option<usize>,
    }

    impl FixtureReader {
        fn new(rows: Vec<ProjectListSummary>) -> Self {
            Self {
                rows: Mutex::new(rows),
                calls: AtomicUsize::new(0),
                data_shifted_after: None,
            }
        }

        fn with_data_shifted_after(mut self, n: usize) -> Self {
            self.data_shifted_after = Some(n);
            self
        }
    }

    impl ProjectListReadHandle for FixtureReader {
        fn read_projects<'a>(&'a self, query: ProjectListQuery) -> ProjectListReadFuture<'a> {
            Box::pin(async move {
                let call_index = self.calls.fetch_add(1, Ordering::SeqCst);
                let mut rows = self.rows.lock().expect("rows lock").clone();
                if let Some(threshold) = self.data_shifted_after {
                    if call_index >= threshold {
                        rows.insert(
                            0,
                            ProjectListSummary {
                                project_id: "project-shifted".to_string(),
                                name: "Newly Added Project".to_string(),
                                parent_account_id: Some("acct-example".to_string()),
                                status: "active".to_string(),
                                trajectory: ProjectTrajectory::Improving,
                                last_touchpoint_at: Some("2026-05-21T00:00:00Z".to_string()),
                            },
                        );
                    }
                }
                if let Some(filter) = query.status.as_deref() {
                    rows.retain(|row| row.status == filter);
                }
                if let Some(filter) = query.trajectory {
                    rows.retain(|row| row.trajectory == filter);
                }
                if let Some(filter) = query.parent_account_id.as_deref() {
                    rows.retain(|row| row.parent_account_id.as_deref() == Some(filter));
                }
                if let Some(needle) = query.name_contains.as_deref() {
                    let needle_lc = needle.to_lowercase();
                    rows.retain(|row| row.name.to_lowercase().contains(&needle_lc));
                }
                let total = rows.len() as u64;
                let offset = query.offset as usize;
                let end = (offset + query.page_size as usize).min(rows.len());
                let items = if offset >= rows.len() {
                    Vec::new()
                } else {
                    rows[offset..end].to_vec()
                };
                let data_shifted_advisory = if matches!(
                    self.data_shifted_after,
                    Some(threshold) if call_index >= threshold
                ) {
                    Some("concurrent insert observed mid-pagination".to_string())
                } else {
                    None
                };
                Ok::<_, ProjectListReadError>(ProjectListSnapshot {
                    items,
                    total_after_filter: total,
                    data_shifted_advisory,
                })
            })
        }
    }

    struct StaticProvider;

    #[async_trait]
    impl IntelligenceProvider for StaticProvider {
        async fn complete(
            &self,
            _prompt: PromptInput,
            _tier: ModelTier,
        ) -> Result<Completion, ProviderError> {
            Ok(Completion {
                text: String::new(),
                fingerprint_metadata: FingerprintMetadata {
                    provider: ProviderKind::Other("test"),
                    model: ModelName::new("unused"),
                    temperature: 0.0,
                    top_p: None,
                    seed: None,
                    tokens_input: None,
                    tokens_output: None,
                    provider_completion_id: None,
                },
            })
        }

        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::Other("test")
        }

        fn current_model(&self, _tier: ModelTier) -> ModelName {
            ModelName::new("unused")
        }
    }

    fn project(name: &str, status: &str, trajectory: ProjectTrajectory) -> ProjectListSummary {
        ProjectListSummary {
            project_id: format!("project-{}", name.replace(' ', "-").to_lowercase()),
            name: name.to_string(),
            parent_account_id: Some("acct-example".to_string()),
            status: status.to_string(),
            trajectory,
            last_touchpoint_at: Some("2026-05-20T12:00:00Z".to_string()),
        }
    }

    fn four_projects() -> Vec<ProjectListSummary> {
        vec![
            project("Rollout Plan", "active", ProjectTrajectory::Improving),
            project("Renewal Path", "active", ProjectTrajectory::Steady),
            project("Migration Spike", "paused", ProjectTrajectory::Degrading),
            project("Expansion Sprint", "active", ProjectTrajectory::Improving),
        ]
    }

    async fn invoke(
        reader: Arc<FixtureReader>,
        actor: Actor,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, crate::abilities::AbilityError> {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 21, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_actor("test")
            .with_project_list_reader(reader);
        let provider = StaticProvider;
        let ctx = crate::abilities::AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            actor,
            None,
            ClaimDismissalSurface::Eval,
        );
        registry
            .invoke_by_name_json(&ctx, ABILITY_NAME, input)
            .await
    }

    fn input_value(filter: Option<serde_json::Value>, page_size: u32) -> serde_json::Value {
        let mut value = json!({ "schemaVersion": 1, "pageSize": page_size });
        if let Some(filter) = filter {
            value["filter"] = filter;
        }
        value
    }

    #[tokio::test]
    async fn returns_paginated_with_valid_cursor_for_first_page() {
        let reader = Arc::new(FixtureReader::new(four_projects()));
        let response = invoke(reader, Actor::User, input_value(None, 2))
            .await
            .expect("first page succeeds");
        let data = &response["data"];
        assert_eq!(data["items"].as_array().unwrap().len(), 2);
        let cursor = data["nextCursor"].as_str().expect("next cursor present");
        let payload = decode_cursor(
            &crate::abilities::get_entity_intelligence::contracts::Cursor::new(cursor),
        )
        .expect("cursor decodes");
        assert_eq!(payload.offset, 2);
    }

    #[tokio::test]
    async fn empty_result_returns_stable_cursor_state() {
        let reader = Arc::new(FixtureReader::new(Vec::new()));
        let response = invoke(reader, Actor::User, input_value(None, 25))
            .await
            .expect("empty case succeeds");
        let data = &response["data"];
        assert_eq!(data["cursorState"]["kind"], "stable");
        assert_eq!(data["items"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn concurrent_insert_during_pagination_returns_data_shifted() {
        let reader = Arc::new(FixtureReader::new(four_projects()).with_data_shifted_after(0));
        let response = invoke(reader, Actor::User, input_value(None, 3))
            .await
            .expect("data-shift case succeeds");
        let data = &response["data"];
        assert_eq!(data["cursorState"]["kind"], "data_shifted");
    }

    #[tokio::test]
    async fn agent_mcp_actor_is_denied() {
        let reader = Arc::new(FixtureReader::new(four_projects()));
        let err = invoke(
            reader,
            Actor::McpClient {
                client_id: McpClientId::new("mcp-test"),
                conversation_handle: None,
            },
            input_value(None, 25),
        )
        .await
        .expect_err("McpClient denied");
        assert_eq!(err.kind, AbilityErrorKind::Capability);
    }

    #[tokio::test]
    async fn user_and_agent_actors_are_allowed() {
        let reader = Arc::new(FixtureReader::new(four_projects()));
        invoke(reader.clone(), Actor::User, input_value(None, 25))
            .await
            .expect("User allowed");
        invoke(reader, Actor::Agent, input_value(None, 25))
            .await
            .expect("Agent allowed");
    }

    #[tokio::test]
    async fn unknown_filter_key_is_rejected() {
        let reader = Arc::new(FixtureReader::new(four_projects()));
        let err = invoke(
            reader,
            Actor::User,
            json!({
                "schemaVersion": 1,
                "pageSize": 25,
                "filter": { "doesNotExist": 1 }
            }),
        )
        .await
        .expect_err("rejects unknown key");
        assert_eq!(err.kind, AbilityErrorKind::Validation);
    }

    #[tokio::test]
    async fn trajectory_filter_round_trips() {
        let reader = Arc::new(FixtureReader::new(four_projects()));
        let response = invoke(
            reader,
            Actor::User,
            json!({
                "schemaVersion": 1,
                "pageSize": 25,
                "filter": { "trajectory": "improving" }
            }),
        )
        .await
        .expect("trajectory filter accepted");
        let data = &response["data"];
        assert_eq!(data["items"].as_array().unwrap().len(), 2);
        for item in data["items"].as_array().unwrap() {
            assert_eq!(item["trajectory"], "improving");
        }
    }
}
