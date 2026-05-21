#[test]
fn assign_inbox_entity_uses_arc_appstate_tauri_state_pattern() {
    let source = std::fs::read_to_string("src/commands/workspace.rs").expect("workspace command");
    let start = source
        .find("pub async fn assign_inbox_entity(")
        .expect("assign_inbox_entity handler exists");
    let body = &source[start..];
    let end = body
        .find("fn assign_inbox_entity_in_db")
        .unwrap_or(body.len());
    let handler = &body[..end];

    assert!(
        handler.contains("state: State<'_, Arc<AppState>>"),
        "assign_inbox_entity must follow live command convention: State<'_, Arc<AppState>>"
    );
    assert!(
        handler.contains(".db_write(move |db|"),
        "assign_inbox_entity must use state.db_write(|db| ...).await"
    );
}
