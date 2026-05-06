use std::process::Command;

#[test]
fn render_policy_coverage_script_passes() {
    let output = Command::new("bash")
        .arg("scripts/check_render_policy_coverage.sh")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run DOS-412 render policy lint");

    assert!(
        output.status.success(),
        "render policy lint failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
