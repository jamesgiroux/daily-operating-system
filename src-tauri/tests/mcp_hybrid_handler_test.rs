#[test]
fn mcp_hybrid_handler_source_preserves_inherent_tool_box_only() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp/main.rs"
    ))
    .expect("read mcp main source");

    assert!(source.contains("#[tool(tool_box)]\nimpl DailyOsMcp"));
    assert!(!source.contains("#[tool(tool_box)]\nimpl ServerHandler for DailyOsMcp"));
    assert!(source.contains("impl ServerHandler for DailyOsMcp"));
}

#[test]
fn mcp_hybrid_handler_source_routes_static_before_ability_bridge() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp/main.rs"
    ))
    .expect("read mcp main source");

    let static_route = source.find("McpToolRoute::Static").unwrap();
    let ability_route = source.find("invoke_mcp_ability_tool").unwrap();

    assert!(static_route < ability_route);
    assert!(source.contains("Self::tool_box().call(context).await"));
    assert!(source.contains(".invoke_ability(session_id, &ability_name"));
}

#[test]
fn mcp_hybrid_call_tool_routes_get_provenance_to_bridge_session_scoped_lookup() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp/main.rs"
    ))
    .expect("read mcp main source");

    assert!(source.contains("McpToolRoute::GetProvenance"));
    assert!(source.contains("invoke_mcp_get_provenance_tool("));
    assert!(source.contains("get_provenance_invocation_id(&request)"));
    assert!(source.contains("self.mcp_session_id"));
    assert!(source.contains(".get_provenance_tool_response(session_id, invocation_id)"));
}

#[test]
fn mcp_hybrid_call_tool_routes_request_confirmation_to_bridge() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp/main.rs"
    ))
    .expect("read mcp main source");

    assert!(source.contains("McpToolRoute::RequestConfirmation"));
    assert!(source.contains("invoke_mcp_request_confirmation_tool("));
    assert!(source.contains("request_confirmation_args(&request)"));
    assert!(source.contains(".request_confirmation_tool(session_id, &ability, &input_json, tauri_bridge)"));
}

#[test]
fn mcp_hybrid_list_tools_includes_get_provenance_with_additional_properties_false_schema() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp/main.rs"
    ))
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
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp/main.rs"
    ))
    .expect("read mcp main source");

    assert!(source.contains("tools.push(request_confirmation_tool_descriptor())"));
    assert!(source.contains("Tool::new(\n        \"request_confirmation\""));
    assert!(source.contains("\"additionalProperties\".to_string()"));
    assert!(source.contains("serde_json::Value::Bool(false)"));
    assert!(source.contains("\"ability\".to_string()"));
    assert!(source.contains("\"input_json\".to_string()"));
}
