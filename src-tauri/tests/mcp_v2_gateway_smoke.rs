#![cfg(feature = "test-harness")]

use std::sync::Arc;

use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::db_service::DbService;
use dailyos_lib::services::mcp_v2::contracts::{
    McpClientId, McpToolRequestEnvelope, McpToolResult, Scope, ScopedName,
};
use dailyos_lib::services::mcp_v2::gateway::McpGateway;
use dailyos_lib::state::AppState;
use serde_json::json;

#[tokio::test]
async fn gateway_dispatches_account_status_with_request_context() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_service = DbService::open_at_unencrypted_for_tests(dir.path().join("mcp-v2-smoke.db"))
        .await
        .expect("db service");

    db_service
        .writer()
        .call(|conn| {
            let db = ActionDb::from_conn(conn);
            let account = DbAccount {
                id: "acct-smoke".to_string(),
                name: "Smoke Account".to_string(),
                lifecycle: Some("active".to_string()),
                health: Some("green".to_string()),
                updated_at: "2026-05-21T12:00:00Z".to_string(),
                ..Default::default()
            };
            Ok(db
                .upsert_account(&account)
                .map_err(|error| error.to_string()))
        })
        .await
        .expect("seed task")
        .expect("seed account");

    let state = Arc::new(AppState::test_with_db_service(db_service));
    let gateway = McpGateway::with_default_handlers(state);
    let response = gateway
        .invoke(
            McpClientId::new("client-smoke"),
            vec![Scope::new("dailyos.read.account_status")],
            McpToolRequestEnvelope {
                conversation_handle: None,
                tool_name: ScopedName::new("dailyos.read.account_status"),
                params: json!({ "accountId": "acct-smoke" }),
            },
        )
        .await;

    let McpToolResult::Ok { value } = response.result else {
        panic!("expected account status result");
    };

    assert_eq!(value["account"]["accountId"], "acct-smoke");
    assert_eq!(value["account"]["name"], "Smoke Account");
    assert_eq!(value["account"]["health"], "green");
}
