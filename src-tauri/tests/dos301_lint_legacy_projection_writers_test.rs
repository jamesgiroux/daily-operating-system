use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn script_root() -> PathBuf {
    let cwd = std::env::current_dir().expect("cwd");
    for dir in cwd.ancestors() {
        let direct = dir.join("scripts/check_dos301_legacy_projection_writers.sh");
        if direct.exists() {
            return dir.to_path_buf();
        }

        let nested = dir
            .join("src-tauri")
            .join("scripts/check_dos301_legacy_projection_writers.sh");
        if nested.exists() {
            return dir.join("src-tauri");
        }
    }
    panic!("locate check_dos301_legacy_projection_writers.sh");
}

#[test]
fn lint_legacy_snapshot_does_not_write_registry_backed_columns() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let services_dir = tmp.path().join("src-tauri/src/services");
    fs::create_dir_all(&services_dir).expect("mkdir services");
    fs::write(
        services_dir.join("derived_state.rs"),
        r#"
pub fn upsert_entity_intelligence_legacy_snapshot() {
    conn.execute(
        "INSERT INTO entity_assessment (entity_id, executive_assessment)
         VALUES (?1, ?2)",
        []
    ).unwrap();
}

pub fn upsert_entity_health_legacy_projection() {}
"#,
    )
    .expect("write fixture");

    let script = script_root().join("scripts/check_dos301_legacy_projection_writers.sh");
    let output = Command::new("bash")
        .arg(script)
        .current_dir(tmp.path())
        .output()
        .expect("run lint");

    assert!(
        !output.status.success(),
        "lint should reject registry-backed legacy snapshot writes"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("executive_assessment"),
        "lint output should name the blocked column, got:\n{stdout}"
    );
}
