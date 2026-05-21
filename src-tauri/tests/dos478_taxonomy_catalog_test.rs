//! Integration tests for DOS-478 W1-B taxonomy catalog: end-to-end
//! load + Gateway::seal handler-binding validation.

use std::sync::Arc;

use dailyos_lib::services::mcp_v2::contracts::{
    McpActor, McpToolHandler, ParamSchema, ReturnSpec, Scope, ScopedName, Side, ToolDescription,
    ToolError, ToolExample,
};
use dailyos_lib::services::mcp_v2::gateway::Gateway;
use dailyos_lib::services::mcp_v2::taxonomy::{
    TaxonomyCatalog, TaxonomyError, YamlTaxonomyCatalog,
};

struct StubHandler {
    description: ToolDescription,
}

impl StubHandler {
    fn new(name: &str, side: Side, scope: &str) -> Self {
        Self {
            description: ToolDescription {
                name: ScopedName::new(name),
                summary: "test".into(),
                when_to_call: "test".into(),
                when_not_to_call: "test".into(),
                side,
                parameters: vec![],
                returns: ReturnSpec {
                    schema: ParamSchema(serde_json::json!({})),
                    description: "test".into(),
                },
                examples: vec![],
                scopes_required: vec![Scope::new(scope)],
            },
        }
    }
}

impl McpToolHandler for StubHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }
    fn invoke(
        &self,
        _: &McpActor,
        _: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        Ok(serde_json::json!({}))
    }
}

#[test]
fn embedded_catalog_loads_and_seal_succeeds_with_full_handler_set() {
    let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded YAML loads");

    let stubs: Vec<Arc<dyn McpToolHandler>> = vec![
        Arc::new(StubHandler::new(
            "dailyos.read.account_status",
            Side::Read,
            "dailyos.read.account_status",
        )),
        Arc::new(StubHandler::new(
            "dailyos.read.daily_briefing",
            Side::Read,
            "dailyos.read.daily_briefing",
        )),
        Arc::new(StubHandler::new(
            "dailyos.read.meeting_briefing",
            Side::Read,
            "dailyos.read.meeting_briefing",
        )),
        Arc::new(StubHandler::new(
            "dailyos.read.portfolio_attention",
            Side::Read,
            "dailyos.read.portfolio_attention",
        )),
        Arc::new(StubHandler::new(
            "dailyos.search.workspace_memory",
            Side::Read,
            "read.workspace_graph",
        )),
        Arc::new(StubHandler::new(
            "dailyos.read.workspace_source_provenance",
            Side::Read,
            "dailyos.read.workspace_source_provenance",
        )),
        Arc::new(StubHandler::new(
            "dailyos.write.place_document",
            Side::Write,
            "write.workspace_place_document",
        )),
        Arc::new(StubHandler::new(
            "dailyos.submit.note",
            Side::SubmitCorrection,
            "dailyos.submit.note",
        )),
        Arc::new(StubHandler::new(
            "dailyos.submit.action",
            Side::SubmitCorrection,
            "dailyos.submit.action",
        )),
        Arc::new(StubHandler::new(
            "dailyos.submit.action_status",
            Side::SubmitCorrection,
            "dailyos.submit.action_status",
        )),
    ];

    let mut gateway = Gateway::new();
    for h in stubs {
        gateway.register(h);
    }
    gateway.set_taxonomy(Arc::new(catalog));
    let pending = gateway.seal().expect("seal succeeds with full handler set");
    assert!(pending.is_empty(), "no pending tools, full set registered: {pending:?}");
}

#[test]
fn seal_fails_on_extra_handler_not_in_catalog() {
    let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded YAML loads");

    let mut gateway = Gateway::new();
    gateway.register(Arc::new(StubHandler::new(
        "dailyos.read.unknown_tool",
        Side::Read,
        "dailyos.read.unknown_tool",
    )));
    gateway.set_taxonomy(Arc::new(catalog));

    match gateway.seal() {
        Err(TaxonomyError::HandlerCatalogMismatch {
            handler,
            catalog_entry,
            nearest_candidate,
        }) => {
            assert_eq!(handler.as_str(), "dailyos.read.unknown_tool");
            assert!(catalog_entry.is_none());
            assert!(
                nearest_candidate.is_some(),
                "nearest_candidate suggests a real tool"
            );
        }
        other => panic!("expected HandlerCatalogMismatch, got {other:?}"),
    }
}

#[test]
fn seal_returns_pending_when_handlers_missing_from_catalog() {
    let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded YAML loads");

    let mut gateway = Gateway::new();
    gateway.register(Arc::new(StubHandler::new(
        "dailyos.read.account_status",
        Side::Read,
        "dailyos.read.account_status",
    )));
    gateway.set_taxonomy(Arc::new(catalog));

    let pending = gateway.seal().expect("seal succeeds (only 1 handler is registered, rest pending)");
    assert_eq!(
        pending.len(),
        9,
        "9 catalog entries have no matching handler (10 catalog - 1 handler): {pending:?}"
    );
}

#[test]
fn seal_fails_on_side_mismatch() {
    let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded YAML loads");

    let mut gateway = Gateway::new();
    // dailyos.read.account_status catalog side = Read; handler advertises Write.
    gateway.register(Arc::new(StubHandler::new(
        "dailyos.read.account_status",
        Side::Write,
        "dailyos.read.account_status",
    )));
    gateway.set_taxonomy(Arc::new(catalog));

    match gateway.seal() {
        Err(TaxonomyError::SideMismatch {
            handler,
            expected,
            actual,
        }) => {
            assert_eq!(handler.as_str(), "dailyos.read.account_status");
            assert_eq!(expected, Side::Read);
            assert_eq!(actual, Side::Write);
        }
        other => panic!("expected SideMismatch, got {other:?}"),
    }
}

#[test]
fn seal_without_taxonomy_is_noop_for_tests() {
    let mut gateway = Gateway::new();
    gateway.register(Arc::new(StubHandler::new(
        "dailyos.read.anything",
        Side::Read,
        "dailyos.read.anything",
    )));
    let pending = gateway.seal().expect("no taxonomy → no validation");
    assert!(pending.is_empty());
}

#[test]
fn load_from_path_round_trip() {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let yaml = r#"
- name: dailyos.read.foo
  side: Read
  summary: x
  when_to_call: y
  when_NOT_to_call: z
  scopesRequired: [dailyos.read.foo]
  parameters: []
  returns: { schema: {}, description: x }
  examples: []
  selectionFixtures:
    positive: [{ prompt: a, expectedTool: dailyos.read.foo }, { prompt: b, expectedTool: dailyos.read.foo }]
    negativeBroadCorpus: [{ prompt: c, expectedToolClass: external }, { prompt: d, expectedToolClass: external }]
    negativeAdjacentTool: [{ prompt: e, expectedTool: dailyos.read.bar }]
"#;
    tmp.write_all(yaml.as_bytes()).expect("write");

    let catalog = YamlTaxonomyCatalog::load_from_path(tmp.path()).expect("load_from_path");
    assert_eq!(catalog.len(), 1);
    assert!(catalog
        .description_for(&ScopedName::new("dailyos.read.foo"))
        .is_some());
}
