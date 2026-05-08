use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    std::env::current_dir()
        .expect("cwd")
        .ancestors()
        .find(|p| p.join("src-tauri/Cargo.toml").exists())
        .expect("repo root with src-tauri/Cargo.toml")
        .to_path_buf()
}

#[test]
fn signal_policy_registry_lint_passes_current_tree() {
    let root = repo_root();
    let output = Command::new("bash")
        .arg(root.join("scripts/check_signal_policy_registry.sh"))
        .output()
        .expect("run signal policy registry lint");

    assert!(
        output.status.success(),
        "signal policy registry lint failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
