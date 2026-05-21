//! `list_accounts` Read ability — paginated account index for the
//! v1.4.4 W2 Accounts list shell (`wp/dailyos/blocks/accounts-index`).
//!
//! W1 substrate extension closing the gap noted in W2 §5.5 L1: the
//! Accounts list shell calls `executeAbility('list_accounts', ...)` via
//! `useAbilityCursor`; this is the producer that fulfills that
//! contract. Returns `Paginated<AccountSummary>` with the same
//! `Cursor` + `CursorState` shape the entity envelope uses (W1 §13 Q11
//! convention) so the shared client-side hook needs no special-casing.
//!
//! Allowed actors: `User` + `Agent`. `AgentMcp` (`Actor::McpClient`) is
//! deliberately excluded per cycle-2 CSO discipline — list abilities are
//! not yet exposed to agent-side MCP, and `mcp_exposure = None` keeps
//! them out of MCP introspection.
//!
//! Per James "every surface in 1.4.4. No deferrals." (2026-05-21).

pub mod contracts;
pub mod producer;

pub use contracts::{AccountListFilter, AccountListInput, AccountSummary};

use dailyos_abilities_macro::ability;

use crate::abilities::get_entity_intelligence::contracts::Paginated;
use crate::abilities::{AbilityContext, AbilityResult};

pub(crate) const ABILITY_NAME: &str = "list_accounts";
pub(crate) const ABILITY_SCHEMA_VERSION: u32 = 1;

#[ability(
    name = "list_accounts",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, Agent, SurfaceClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.accounts_list"],
    mcp_exposure = None,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn list_accounts(
    ctx: &AbilityContext<'_>,
    input: AccountListInput,
) -> AbilityResult<Paginated<AccountSummary>> {
    producer::list_accounts(ctx, input).await
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
    use crate::abilities::list_pagination::{decode_cursor, encode_cursor, watermark_from_request};
    use crate::abilities::registry::{AbilityRegistry, McpClientId};
    use crate::abilities::trust::types::TrustBand;
    use crate::abilities::{AbilityErrorKind, Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
        ProviderError, ProviderKind,
    };
    use crate::sensitivity::ClaimDismissalSurface;
    use crate::services::context::{
        AccountListQuery, AccountListReadError, AccountListReadFuture, AccountListReadHandle,
        AccountListSnapshot, AccountListSummary, FixedClock, SeedableRng, ServiceContext,
    };

    struct FixtureReader {
        rows: Mutex<Vec<AccountListSummary>>,
        calls: AtomicUsize,
        data_shifted_after: Option<usize>,
    }

    impl FixtureReader {
        fn new(rows: Vec<AccountListSummary>) -> Self {
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

    impl AccountListReadHandle for FixtureReader {
        fn read_accounts<'a>(&'a self, query: AccountListQuery) -> AccountListReadFuture<'a> {
            Box::pin(async move {
                let call_index = self.calls.fetch_add(1, Ordering::SeqCst);
                let mut rows = self.rows.lock().expect("rows lock").clone();
                if let Some(threshold) = self.data_shifted_after {
                    if call_index >= threshold {
                        // Simulate a concurrent insert by shifting an extra row in.
                        rows.insert(
                            0,
                            AccountListSummary {
                                account_id: "acct-shifted".to_string(),
                                name: "Shifted Concurrent Insert".to_string(),
                                status: "active".to_string(),
                                health_band: TrustBand::LikelyCurrent,
                                last_touchpoint_at: Some("2026-05-21T00:00:00Z".to_string()),
                                open_loops_count: 0,
                            },
                        );
                    }
                }
                if let Some(filter) = query.status.as_deref() {
                    rows.retain(|row| row.status == filter);
                }
                if let Some(filter) = query.health_band {
                    rows.retain(|row| row.health_band == filter);
                }
                if let Some(needle) = query.name_contains.as_deref() {
                    let needle_lc = needle.to_lowercase();
                    rows.retain(|row| row.name.to_lowercase().contains(&needle_lc));
                }
                let total = rows.len() as u64;
                let offset = query.offset as usize;
                let end =
                    (offset + query.page_size as usize).min(rows.len());
                let items = if offset >= rows.len() {
                    Vec::new()
                } else {
                    rows[offset..end].to_vec()
                };
                let data_shifted_advisory = if matches!(self.data_shifted_after, Some(threshold) if call_index >= threshold)
                {
                    Some("concurrent insert observed mid-pagination".to_string())
                } else {
                    None
                };
                Ok::<_, AccountListReadError>(AccountListSnapshot {
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

    fn account(name: &str, status: &str, band: TrustBand) -> AccountListSummary {
        AccountListSummary {
            account_id: format!("acct-{}", name.replace(' ', "-").to_lowercase()),
            name: name.to_string(),
            status: status.to_string(),
            health_band: band,
            last_touchpoint_at: Some("2026-05-20T12:00:00Z".to_string()),
            open_loops_count: 2,
        }
    }

    fn five_accounts() -> Vec<AccountListSummary> {
        vec![
            account("Alpha Example", "active", TrustBand::LikelyCurrent),
            account("Beta Example", "active", TrustBand::UseWithCaution),
            account("Gamma Example", "paused", TrustBand::NeedsVerification),
            account("Delta Example", "active", TrustBand::LikelyCurrent),
            account("Epsilon Example", "active", TrustBand::LikelyCurrent),
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
            .with_account_list_reader(reader);
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
        let reader = Arc::new(FixtureReader::new(five_accounts()));
        let response = invoke(reader, Actor::User, input_value(None, 2))
            .await
            .expect("first page succeeds");

        let data = &response["data"];
        assert_eq!(data["items"].as_array().unwrap().len(), 2);
        assert_eq!(data["totalHint"].as_u64().unwrap(), 5);
        assert_eq!(data["cursorState"]["kind"], "stable");
        let cursor = data["nextCursor"].as_str().expect("next cursor present");
        let payload = decode_cursor(&crate::abilities::get_entity_intelligence::contracts::Cursor::new(cursor))
            .expect("cursor decodes");
        assert_eq!(payload.offset, 2);
        let expected_watermark = watermark_from_request(
            &json!({
                "status": null,
                "health_band": null,
                "name_contains": null,
                "page_size": 2u32,
                "ability": ABILITY_NAME,
            })
            .to_string(),
        );
        assert_eq!(payload.watermark, expected_watermark);
    }

    #[tokio::test]
    async fn empty_result_returns_stable_cursor_state() {
        let reader = Arc::new(FixtureReader::new(Vec::new()));
        let response = invoke(reader, Actor::User, input_value(None, 25))
            .await
            .expect("empty list succeeds");

        let data = &response["data"];
        assert_eq!(data["items"].as_array().unwrap().len(), 0);
        assert_eq!(data["totalHint"].as_u64().unwrap(), 0);
        assert_eq!(data["cursorState"]["kind"], "stable");
        assert!(data["nextCursor"].is_null());
    }

    #[tokio::test]
    async fn concurrent_insert_during_pagination_returns_data_shifted() {
        let reader = Arc::new(FixtureReader::new(five_accounts()).with_data_shifted_after(0));
        let response = invoke(reader, Actor::User, input_value(None, 3))
            .await
            .expect("data shift call succeeds");

        let data = &response["data"];
        assert_eq!(data["cursorState"]["kind"], "data_shifted");
        let advisory = data["cursorState"]["advisory"]
            .as_str()
            .expect("advisory present");
        assert!(advisory.contains("concurrent insert"));
    }

    #[tokio::test]
    async fn malformed_cursor_returns_invalidated_restart_required() {
        let reader = Arc::new(FixtureReader::new(five_accounts()));
        let response = invoke(
            reader,
            Actor::User,
            json!({
                "schemaVersion": 1,
                "pageSize": 3,
                "cursor": "!!!not-base64!!!"
            }),
        )
        .await
        .expect("invalidated path still produces an envelope");

        let data = &response["data"];
        assert_eq!(data["cursorState"]["kind"], "invalidated");
        assert!(
            data["cursorState"]["restart_required"].as_bool().unwrap(),
            "restart_required must be true"
        );
    }

    #[tokio::test]
    async fn watermark_mismatch_returns_invalidated_restart_required() {
        let reader = Arc::new(FixtureReader::new(five_accounts()));
        // Cursor watermark belongs to a different filter than what we send.
        let cursor =
            encode_cursor(2, "0000000000000000");
        let response = invoke(
            reader,
            Actor::User,
            json!({
                "schemaVersion": 1,
                "pageSize": 2,
                "cursor": cursor.as_str(),
            }),
        )
        .await
        .expect("watermark mismatch produces an envelope");

        let data = &response["data"];
        assert_eq!(data["cursorState"]["kind"], "invalidated");
        assert!(data["cursorState"]["restart_required"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn agent_mcp_actor_is_denied() {
        let reader = Arc::new(FixtureReader::new(five_accounts()));
        let err = invoke(
            reader,
            Actor::McpClient {
                client_id: McpClientId::new("mcp-test"),
                conversation_handle: None,
            },
            input_value(None, 25),
        )
        .await
        .expect_err("McpClient is denied");

        assert_eq!(err.kind, AbilityErrorKind::Capability);
    }

    #[tokio::test]
    async fn user_and_agent_actors_are_allowed() {
        let reader = Arc::new(FixtureReader::new(five_accounts()));
        invoke(reader.clone(), Actor::User, input_value(None, 25))
            .await
            .expect("User is allowed");
        invoke(reader, Actor::Agent, input_value(None, 25))
            .await
            .expect("Agent is allowed");
    }

    #[tokio::test]
    async fn unknown_filter_key_is_rejected() {
        let reader = Arc::new(FixtureReader::new(five_accounts()));
        let err = invoke(
            reader,
            Actor::User,
            json!({
                "schemaVersion": 1,
                "pageSize": 25,
                "filter": { "totallyBogusKey": "x" }
            }),
        )
        .await
        .expect_err("unknown filter key must reject");

        assert_eq!(err.kind, AbilityErrorKind::Validation);
    }

    #[tokio::test]
    async fn final_page_returns_no_next_cursor() {
        let reader = Arc::new(FixtureReader::new(five_accounts()));
        let watermark = watermark_from_request(
            &json!({
                "status": null,
                "health_band": null,
                "name_contains": null,
                "page_size": 2u32,
                "ability": ABILITY_NAME,
            })
            .to_string(),
        );
        let cursor = encode_cursor(4, &watermark);
        let response = invoke(
            reader,
            Actor::User,
            json!({
                "schemaVersion": 1,
                "pageSize": 2,
                "cursor": cursor.as_str(),
            }),
        )
        .await
        .expect("third page succeeds");

        let data = &response["data"];
        assert_eq!(data["items"].as_array().unwrap().len(), 1);
        assert!(data["nextCursor"].is_null());
        assert_eq!(data["cursorState"]["kind"], "stable");
    }
}
