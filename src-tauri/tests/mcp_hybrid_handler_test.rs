#[test]
fn mcp_hybrid_handler_source_preserves_inherent_tool_box_only() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp/main.rs"))
        .expect("read mcp main source");

    assert!(source.contains("#[tool(tool_box)]\nimpl DailyOsMcp"));
    assert!(!source.contains("#[tool(tool_box)]\nimpl ServerHandler for DailyOsMcp"));
    assert!(source.contains("impl ServerHandler for DailyOsMcp"));
}

#[test]
fn mcp_hybrid_handler_source_routes_static_before_ability_bridge() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp/main.rs"))
        .expect("read mcp main source");

    let static_route = source.find("McpToolRoute::Static").unwrap();
    let ability_route = source.find("invoke_mcp_ability_tool").unwrap();

    assert!(static_route < ability_route);
    assert!(source.contains("Self::tool_box().call(context).await"));
    assert!(source.contains(".invoke_ability(session_id, &ability_name"));
}

#[test]
fn mcp_hybrid_call_tool_routes_get_provenance_to_bridge_session_scoped_lookup() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp/main.rs"))
        .expect("read mcp main source");

    assert!(source.contains("McpToolRoute::GetProvenance"));
    assert!(source.contains("invoke_mcp_get_provenance_tool("));
    assert!(source.contains("get_provenance_invocation_id(&request)"));
    assert!(source.contains("self.mcp_session_id"));
    assert!(source.contains(".get_provenance_tool_response(session_id, invocation_id)"));
}

#[test]
fn mcp_hybrid_call_tool_routes_request_confirmation_to_bridge() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp/main.rs"))
        .expect("read mcp main source");

    assert!(source.contains("McpToolRoute::RequestConfirmation"));
    assert!(source.contains("invoke_mcp_request_confirmation_tool("));
    assert!(source.contains("request_confirmation_args(&request)"));
    assert!(source
        .contains(".request_confirmation_tool(session_id, &ability, &input_json, tauri_bridge)"));
}

#[test]
fn mcp_hybrid_list_tools_includes_get_provenance_with_additional_properties_false_schema() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp/main.rs"))
        .expect("read mcp main source");

    assert!(source.contains("tools.push(get_provenance_tool_descriptor())"));
    assert!(source.contains("Tool::new(\n        \"get_provenance\""));
    assert!(source.contains("\"additionalProperties\".to_string()"));
    assert!(source.contains("serde_json::Value::Bool(false)"));
    assert!(source.contains("\"required\".to_string()"));
    assert!(source.contains("\"invocation_id\".to_string()"));
}

#[test]
fn mcp_hybrid_list_tools_includes_request_confirmation_with_closed_schema() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp/main.rs"))
        .expect("read mcp main source");

    assert!(source.contains("tools.push(request_confirmation_tool_descriptor())"));
    assert!(source.contains("Tool::new(\n        \"request_confirmation\""));
    assert!(source.contains("\"additionalProperties\".to_string()"));
    assert!(source.contains("serde_json::Value::Bool(false)"));
    assert!(source.contains("\"ability\".to_string()"));
    assert!(source.contains("\"input_json\".to_string()"));
}

#[cfg(feature = "mcp")]
mod mcp_open_schema_runtime {
    use std::future::Future;
    use std::pin::Pin;

    use dailyos_lib::abilities::registry::{AbilityPolicy, McpExposure, SignalPolicy};
    use dailyos_lib::abilities::{
        AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, Actor,
    };
    use dailyos_lib::bridges::mcp::McpAbilityBridge;
    use dailyos_lib::bridges::McpSessionId;
    use dailyos_lib::services::context::ExecutionMode;
    use serde_json::json;

    const AGENT_ACTORS: &[Actor] = &[Actor::Agent];
    const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    fn success_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Ok(json!({
                "data": {
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str()
                },
                "ability_version": { "major": 1, "minor": 0 },
                "diagnostics": { "warnings": [] },
                "provenance": {
                    "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                    "ability_name": "fixture",
                    "ability_version": { "major": 1, "minor": 0 },
                    "ability_schema_version": 1,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                    "warnings": []
                }
            }))
        })
    }

    fn open_object_schema() -> serde_json::Value {
        json!({ "type": "object" })
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false
        })
    }

    fn descriptor(name: &'static str) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category: AbilityCategory::Read,
            policy: AbilityPolicy {
                allowed_actors: AGENT_ACTORS,
                allowed_modes: LIVE_MODES,
                requires_confirmation: false,
                may_publish: false,
                required_scopes: &[],
                mcp_exposure: McpExposure::None,
                client_side_executable: false,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy::default(),
            invoke_erased: success_erased,
            input_schema: open_object_schema,
            output_schema: closed_object_schema,
        }
    }

    fn session(index: u128) -> McpSessionId {
        McpSessionId::from_uuid(uuid::Uuid::from_u128(index))
    }

    async fn error_bytes_for(registry: AbilityRegistry, ability_name: &'static str) -> Vec<u8> {
        let bridge = McpAbilityBridge::new(&registry);
        let err = bridge
            .invoke_ability(session(1), ability_name, json!({}), false, None)
            .await
            .unwrap_err();
        serde_json::to_vec(&err).unwrap()
    }

    #[tokio::test]
    async fn mcp_call_tool_open_schema_descriptor_yields_byte_equal_unavailable() {
        let unknown = error_bytes_for(
            AbilityRegistry::from_descriptors_checked(vec![]).unwrap(),
            "unknown",
        )
        .await;
        let open_schema = error_bytes_for(
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                descriptor("open_runtime_schema"),
            ]),
            "open_runtime_schema",
        )
        .await;

        assert_eq!(open_schema, unknown);
        assert_eq!(open_schema, br#""ability_unavailable""#);
    }
}
