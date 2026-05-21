use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::services::mcp_v2::contracts::{
    compose_scope, compose_scoped_name, McpActor, McpHandlerContext, McpToolHandler, ParamSchema,
    ReturnSpec, Side, ToolDescription, ToolError, ToolExample, Verb,
};

const ACCOUNT_STATUS_NOUN: &str = "account_status";

#[derive(Debug, Clone)]
pub struct AccountStatusToolHandler {
    description: ToolDescription,
}

impl Default for AccountStatusToolHandler {
    fn default() -> Self {
        Self {
            description: account_status_description(),
        }
    }
}

impl McpToolHandler for AccountStatusToolHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(
        &self,
        ctx: &McpHandlerContext<'_>,
        _actor: &McpActor,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let params = AccountStatusParams::try_from_value(params)?;
        let generated_at = ctx.services().clock.now().to_rfc3339();
        let account = ctx.read_db(move |db| {
            let account = match params.lookup {
                AccountStatusLookup::Id(account_id) => db
                    .get_account(&account_id)
                    .map_err(db_error)?
                    .ok_or_else(|| account_not_found(&account_id))?,
                AccountStatusLookup::Name(account_name) => db
                    .get_account_by_name(&account_name)
                    .map_err(db_error)?
                    .ok_or_else(|| account_not_found(&account_name))?,
            };
            let renewal_stage = db
                .get_account_renewal_stage(&account.id)
                .map_err(db_error)?;
            let open_action_count = db.get_account_actions(&account.id).map_err(db_error)?.len();

            Ok(AccountStatusPayload {
                account_id: account.id,
                name: account.name,
                lifecycle: account.lifecycle,
                health: account.health,
                customer_status: account.customer_status,
                renewal_date: account.contract_end,
                renewal_stage,
                account_type: account.account_type.as_db_str().to_string(),
                archived: account.archived,
                open_action_count,
            })
        })?;

        serde_json::to_value(AccountStatusResponse {
            generated_at,
            account,
        })
        .map_err(|_| ToolError::Internal {
            trace_id: format!("mcp-v2-{}", uuid::Uuid::new_v4()),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawAccountStatusParams {
    account_id: Option<String>,
    account_name: Option<String>,
}

struct AccountStatusParams {
    lookup: AccountStatusLookup,
}

enum AccountStatusLookup {
    Id(String),
    Name(String),
}

impl AccountStatusParams {
    fn try_from_value(value: serde_json::Value) -> Result<Self, ToolError> {
        let raw: RawAccountStatusParams =
            serde_json::from_value(value).map_err(|error| ToolError::BadParams {
                detail: error.to_string(),
            })?;

        let account_id = normalize_optional(raw.account_id);
        let account_name = normalize_optional(raw.account_name);
        match (account_id, account_name) {
            (Some(account_id), None) => Ok(Self {
                lookup: AccountStatusLookup::Id(account_id),
            }),
            (None, Some(account_name)) => Ok(Self {
                lookup: AccountStatusLookup::Name(account_name),
            }),
            (None, None) => Err(ToolError::BadParams {
                detail: "provide exactly one of accountId or accountName".to_string(),
            }),
            (Some(_), Some(_)) => Err(ToolError::BadParams {
                detail: "accountId and accountName are mutually exclusive".to_string(),
            }),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AccountStatusResponse {
    generated_at: String,
    account: AccountStatusPayload,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AccountStatusPayload {
    account_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lifecycle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    renewal_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    renewal_stage: Option<String>,
    account_type: String,
    archived: bool,
    open_action_count: usize,
}

fn account_status_description() -> ToolDescription {
    ToolDescription {
        name: compose_scoped_name(Verb::Read, ACCOUNT_STATUS_NOUN),
        summary: "Returns the current status snapshot for one account.".to_string(),
        when_to_call: "Use when the user asks for the status of a specific account.".to_string(),
        when_not_to_call: "Do NOT use for broad portfolio search or updates.".to_string(),
        side: Side::Read,
        parameters: vec![
            crate::services::mcp_v2::contracts::ParamSpec {
                name: "accountId".to_string(),
                schema: ParamSchema(json!({ "type": "string", "minLength": 1 })),
                required: false,
                description: "Stable account identifier.".to_string(),
            },
            crate::services::mcp_v2::contracts::ParamSpec {
                name: "accountName".to_string(),
                schema: ParamSchema(json!({ "type": "string", "minLength": 1 })),
                required: false,
                description: "Exact account name, case-insensitive.".to_string(),
            },
        ],
        returns: ReturnSpec {
            schema: ParamSchema(json!({
                "type": "object",
                "required": ["generatedAt", "account"],
                "properties": {
                    "generatedAt": { "type": "string" },
                    "account": { "type": "object" }
                }
            })),
            description: "Account status snapshot with lifecycle, health, and open action count."
                .to_string(),
        },
        examples: vec![ToolExample {
            prompt: "What is the status of Acme?".to_string(),
            invocation: json!({
                "toolName": "dailyos.read.account_status",
                "params": { "accountName": "Acme" }
            }),
            expected_response_shape: json!({
                "generatedAt": "string",
                "account": {
                    "accountId": "string",
                    "name": "string",
                    "openActionCount": 0
                }
            }),
        }],
        scopes_required: vec![compose_scope(Verb::Read, ACCOUNT_STATUS_NOUN)],
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn db_error(error: crate::db::DbError) -> ToolError {
    ToolError::UpstreamFailure {
        detail: format!("account status read failed: {error}"),
    }
}

fn account_not_found(identifier: &str) -> ToolError {
    ToolError::NotFound {
        resource: format!("account:{identifier}"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::db::{ActionDb, DbAccount};
    use crate::db_service::DbService;
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, Scope, ScopedName,
    };
    use crate::state::AppState;

    struct Fixture {
        _dir: TempDir,
        state: Arc<AppState>,
        handler: AccountStatusToolHandler,
    }

    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_service = DbService::open_at_unencrypted(dir.path().join("account-status.db"))
            .await
            .expect("db service");

        db_service
            .writer()
            .call(|conn| {
                let db = ActionDb::from_conn(conn);
                let account = DbAccount {
                    id: "acct-example".to_string(),
                    name: "Example Account".to_string(),
                    lifecycle: Some("renewing".to_string()),
                    health: Some("yellow".to_string()),
                    contract_end: Some("2026-12-31".to_string()),
                    updated_at: "2026-05-21T12:00:00Z".to_string(),
                    ..Default::default()
                };
                if let Err(error) = db.upsert_account(&account) {
                    return Ok(Err(error.to_string()));
                }
                conn.execute(
                    "UPDATE accounts SET customer_status = 'active' WHERE id = 'acct-example'",
                    [],
                )?;
                Ok(Ok(()))
            })
            .await
            .expect("seed task")
            .expect("seed account");

        Fixture {
            _dir: dir,
            state: Arc::new(AppState::test_with_db_service(db_service)),
            handler: AccountStatusToolHandler::default(),
        }
    }

    fn actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("client-test"),
            conversation_handle: Some(OpaqueConversationHandle::new("conversation-test")),
            tool_name: ScopedName::new("dailyos.read.account_status"),
            granted_scopes: vec![Scope::new("dailyos.read.account_status")],
        }
    }

    async fn invoke(
        fixture: &Fixture,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let ctx = McpHandlerContext::from_state(&fixture.state)
            .await
            .expect("context");
        fixture.handler.invoke(&ctx, &actor(), params)
    }

    #[tokio::test]
    async fn description_declares_account_status_scope() {
        let fixture = fixture().await;
        let description = fixture.handler.description();

        assert_eq!(description.name.as_str(), "dailyos.read.account_status");
        assert_eq!(
            description.scopes_required,
            vec![Scope::new("dailyos.read.account_status")]
        );
    }

    #[tokio::test]
    async fn invoke_rejects_missing_account_identifier() {
        let fixture = fixture().await;
        let err = invoke(&fixture, json!({})).await.unwrap_err();

        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[tokio::test]
    async fn invoke_rejects_multiple_account_identifiers() {
        let fixture = fixture().await;
        let err = invoke(
            &fixture,
            json!({ "accountId": "acct-example", "accountName": "Example Account" }),
        )
        .await
        .unwrap_err();

        assert_eq!(
            err,
            ToolError::BadParams {
                detail: "accountId and accountName are mutually exclusive".to_string()
            }
        );
    }

    #[tokio::test]
    async fn invoke_returns_not_found_for_unknown_account() {
        let fixture = fixture().await;
        let err = invoke(&fixture, json!({ "accountId": "missing-account" }))
            .await
            .unwrap_err();

        assert_eq!(
            err,
            ToolError::NotFound {
                resource: "account:missing-account".to_string()
            }
        );
    }

    #[tokio::test]
    async fn invoke_reads_account_by_id_from_context() {
        let fixture = fixture().await;
        let value = invoke(&fixture, json!({ "accountId": "acct-example" }))
            .await
            .expect("account status");

        assert_eq!(value["account"]["accountId"], "acct-example");
        assert_eq!(value["account"]["name"], "Example Account");
        assert_eq!(value["account"]["health"], "yellow");
        assert_eq!(value["account"]["openActionCount"], 0);
    }

    #[tokio::test]
    async fn invoke_reads_account_by_name_and_shapes_status() {
        let fixture = fixture().await;
        let value = invoke(&fixture, json!({ "accountName": "example account" }))
            .await
            .expect("account status");

        assert_eq!(value["account"]["accountId"], "acct-example");
        assert_eq!(value["account"]["lifecycle"], "renewing");
        assert_eq!(value["account"]["customerStatus"], "active");
        assert_eq!(value["account"]["renewalDate"], "2026-12-31");
        assert_eq!(value["account"]["accountType"], "customer");
    }
}
