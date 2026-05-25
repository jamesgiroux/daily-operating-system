use chrono::Utc;
use dailyos_lib::services::workspace_ingestion::lifecycle::WorkspaceFileLifecycle;

fn main() {
    let now = Utc::now();
    let _ = WorkspaceFileLifecycle {
        file_id: "wf-trybuild".to_string(),
        canonical_path: "/tmp/workspace/file.md".to_string(),
        device: 1,
        inode: 2,
        entity_id: None,
        entity_type: None,
        content_sha256: None,
        category: None,
        user_override: None,
        created_at: now,
        updated_at: now,
    };
}
