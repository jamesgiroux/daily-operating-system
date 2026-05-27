use std::path::PathBuf;

fn manifest_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(manifest_root().join(relative))
        .unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

#[test]
fn mcp_comparison_path_rejects_legacy_authority() {
    let comparison_path_files = [
        "src/bridges/mcp.rs",
        "src/services/mcp_v2/handlers/tool_account_status.rs",
        "src/services/mcp_v2/handlers/tool_placement.rs",
        "src/services/mcp_v2/runtime_projection.rs",
        "src/services/mcp_v2/transport.rs",
    ];
    let forbidden_authority_needles = [
        "db.get_entity_intelligence",
        "entity_assessment",
        "reports.content_json",
        "content_json",
        "prep_frozen_json",
        "risk-briefing.json",
        "dashboard.json",
        "dashboard.md",
        "_today/data",
        "intelligence.json",
    ];

    for relative in comparison_path_files {
        let source = read(relative);
        for needle in forbidden_authority_needles {
            assert!(
                !source.contains(needle),
                "{relative} must not use legacy authority source `{needle}` in the MCP v2 comparison path"
            );
        }
    }
}
