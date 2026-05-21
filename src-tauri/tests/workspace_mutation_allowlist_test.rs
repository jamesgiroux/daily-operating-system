use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn workspace_mutation_allowlist_script_exists_is_executable_and_passes() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_workspace_mutation_allowlist.sh");
    let meta = std::fs::metadata(&script).expect("script metadata");
    assert_ne!(
        meta.permissions().mode() & 0o111,
        0,
        "script must be executable"
    );

    let output = Command::new("bash")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run workspace mutation allowlist");
    assert!(
        output.status.success(),
        "allowlist failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
