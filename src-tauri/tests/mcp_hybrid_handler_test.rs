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
