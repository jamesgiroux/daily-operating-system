use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn run_lint(current_dir: &Path) -> (bool, String, String) {
    let script = repo_root().join("src-tauri/scripts/check_ability_surface_drift.sh");
    assert!(script.is_file(), "missing lint script: {}", script.display());
    let output = Command::new("bash")
        .arg(script)
        .current_dir(current_dir)
        .output()
        .expect("run ability surface drift lint");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn ability_surface_drift_lint_passes_current_tree() {
    let root = repo_root();
    let (ok, stdout, stderr) = run_lint(&root);

    assert!(
        ok,
        "ability surface drift lint failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn surface_drift_lint_blocks_new_capability_tauri_command() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let commands_dir = tmp.path().join("src-tauri/src/commands");
    std::fs::create_dir_all(&commands_dir).expect("mkdir commands");
    std::fs::write(
        commands_dir.join("new_capability.rs"),
        "#[tauri::command]\npub async fn create_customer_record() {}\n",
    )
    .expect("write synthetic command");

    let (ok, stdout, stderr) = run_lint(tmp.path());

    assert!(
        !ok,
        "lint must fail for a new hand-written Tauri command. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(stdout.contains("new_capability.rs"), "stdout: {stdout}");
    assert!(
        stdout.contains("create_customer_record")
            || stdout.contains("new hand-written Tauri command file"),
        "stdout: {stdout}"
    );
}

#[test]
fn surface_drift_lint_blocks_new_handwritten_mcp_tool() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mcp_dir = tmp.path().join("src-tauri/src/mcp");
    std::fs::create_dir_all(&mcp_dir).expect("mkdir mcp");
    std::fs::write(
        mcp_dir.join("main.rs"),
        r#"
#[tool(tool_box)]
impl DailyOsMcp {
    #[tool(description = "Allowed static read")]
    fn get_briefing(&self) -> String { String::new() }

    #[tool(description = "Synthetic drift")]
    fn mutate_entity(&self) -> String { String::new() }
}
"#,
    )
    .expect("write synthetic mcp main");

    let (ok, stdout, stderr) = run_lint(tmp.path());

    assert!(
        !ok,
        "lint must fail for a new hand-written MCP tool. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(stdout.contains("mutate_entity"), "stdout: {stdout}");
}
