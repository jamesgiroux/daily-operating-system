use std::future::Future;
use std::pin::Pin;

use dailyos_lib::abilities::registry::{AbilityPolicy, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, Actor,
};
use dailyos_lib::bridges::mcp::McpAbilityBridge;
use dailyos_lib::bridges::tauri::TauriAbilityBridge;
use dailyos_lib::bridges::McpSessionId;
use dailyos_lib::services::context::ExecutionMode;
use dailyos_lib::state::AppState;
use serde_json::{json, Value};

const PUBLIC_TEXT: &str = "Public launch readiness is green.";
const INTERNAL_TEXT: &str = "Internal rollout dependency is tracked.";
const CONFIDENTIAL_TEXT: &str = "Confidential renewal risk must stay hidden.";
const USER_ONLY_TEXT: &str = "User-only negotiation note must stay hidden.";

const USER_AGENT_ACTORS: &[Actor] = &[Actor::User, Actor::Agent];
const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];

type ErasedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

fn synthetic_claim_text_erased<'a>(
    ctx: &'a AbilityContext<'a>,
    _input: serde_json::Value,
) -> ErasedFuture<'a> {
    Box::pin(async move {
        Ok(json!({
            "data": {
                "claims": {
                    "public": tagged_claim("claim-public", "public", PUBLIC_TEXT),
                    "internal": tagged_claim("claim-internal", "internal", INTERNAL_TEXT),
                    "confidential": tagged_claim(
                        "claim-confidential",
                        "confidential",
                        CONFIDENTIAL_TEXT
                    ),
                    "user_only": tagged_claim("claim-user-only", "user_only", USER_ONLY_TEXT)
                },
                "untagged": {
                    "summary": CONFIDENTIAL_TEXT
                }
            },
            "ability_version": { "major": 1, "minor": 0 },
            "diagnostics": { "warnings": [] },
            "provenance": {
                "invocation_id": "41241241-4124-4124-8124-412412412412",
                "ability_name": "dos412_synthetic_claim_text",
                "ability_version": { "major": 1, "minor": 0 },
                "ability_schema_version": 1,
                "actor": format!("{:?}", ctx.actor),
                "mode": ctx.mode().as_str(),
                "warnings": []
            }
        }))
    })
}

#[tokio::test]
async fn mcp_ability_data_redacts_tagged_private_claim_text_while_tauri_stays_raw() {
    let registry = AbilityRegistry::from_descriptors_checked(vec![synthetic_descriptor()]).unwrap();

    let mcp_bridge = McpAbilityBridge::new(&registry);
    let mcp_response = mcp_bridge
        .invoke_ability(
            McpSessionId::from_uuid(uuid::Uuid::from_u128(412)),
            "dos412_synthetic_claim_text",
            json!({}),
            false,
            None,
        )
        .await
        .unwrap();
    let mcp_data = serde_json::to_string(&mcp_response.data).unwrap();

    assert!(mcp_data.contains(PUBLIC_TEXT));
    assert!(mcp_data.contains(INTERNAL_TEXT));
    assert!(!mcp_data.contains(CONFIDENTIAL_TEXT));
    assert!(!mcp_data.contains(USER_ONLY_TEXT));
    assert!(mcp_response.data["claims"].get("confidential").is_none());
    assert!(mcp_response.data["claims"].get("user_only").is_none());
    assert!(mcp_response.data["untagged"].get("summary").is_none());

    let state = AppState::new();
    let tauri_bridge = TauriAbilityBridge::new(&registry);
    let tauri_response = tauri_bridge
        .invoke(
            &state,
            "dos412_synthetic_claim_text",
            json!({}),
            false,
            None,
        )
        .await
        .unwrap();
    let tauri_data = serde_json::to_string(&tauri_response.data).unwrap();

    for text in [
        PUBLIC_TEXT,
        INTERNAL_TEXT,
        CONFIDENTIAL_TEXT,
        USER_ONLY_TEXT,
    ] {
        assert!(
            tauri_data.contains(text),
            "Tauri data should include {text}"
        );
    }
}

fn tagged_claim(claim_id: &str, sensitivity: &str, text: &str) -> Value {
    json!({
        "text": text,
        "claim_id": claim_id,
        "sensitivity": sensitivity,
        "originating_actor": "user"
    })
}

fn synthetic_descriptor() -> AbilityDescriptor {
    AbilityDescriptor {
        name: "dos412_synthetic_claim_text",
        version: "1.0.0",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: USER_AGENT_ACTORS,
            allowed_modes: LIVE_MODES,
            requires_confirmation: false,
            may_publish: false,
        },
        composes: &[],
        mutates: &[],
        experimental: false,
        registered_at: None,
        signal_policy: SignalPolicy::default(),
        invoke_erased: synthetic_claim_text_erased,
        input_schema: closed_object_schema,
        output_schema: closed_object_schema,
    }
}

fn closed_object_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {}
    })
}
