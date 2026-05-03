//! Provider clock/RNG lint wiring into `cargo test`.
//!
//! Per L2 codex review 2026-04-30 finding #4: a standalone bash script
//! is not part of the merge gate (`cargo clippy && cargo test && pnpm
//! tsc --noEmit`). This integration test invokes the script and asserts
//! exit 0 so the invariant fails CI when violated.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn dos259_provider_clock_rng_lint_passes() {
    let root = workspace_root();
    let script = root.join("scripts/check_no_direct_clock_rng_in_provider_modules.sh");
    assert!(
        script.exists(),
        "DOS-259 lint script missing at {}",
        script.display()
    );

    let output = Command::new("bash")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("execute DOS-259 lint script");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "DOS-259 provider clock/RNG lint failed:\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
        );
    }
}

#[test]
fn dos259_provider_clock_rng_lint_catches_unmarked_violation() {
    // L2 cycle-2 finding #3 regression: the lint must actually trip on
    // a marker-less Utc::now() / thread_rng() call. Earlier version of
    // this test only verified marker strings existed in the script —
    // codex correctly flagged that as not exercising the matching logic.
    //
    // This version writes a synthetic provider module to a temp file
    // with a bare `chrono::Utc::now()` call (no exempt/grandfathered
    // marker) and invokes the script with the
    // DOS259_LINT_FILES_OVERRIDE env var pointing at the temp file.
    // Asserts the script exits non-zero, confirming the lint matching
    // logic actually catches violations.
    let tmp_dir = std::env::temp_dir().join(format!("dos259-lint-test-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).expect("create temp dir");
    let fixture = tmp_dir.join("synthetic_unmarked_provider.rs");
    let synthetic = "\
//! Synthetic provider-module fixture for the clock/RNG lint test.
//! Contains an UNMARKED `chrono::Utc::now()` call — the lint must trip.
fn timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}
";
    std::fs::write(&fixture, synthetic).expect("write fixture");

    let root = workspace_root();
    let script = root.join("scripts/check_no_direct_clock_rng_in_provider_modules.sh");
    let output = Command::new("bash")
        .arg(&script)
        .env("DOS259_LINT_FILES_OVERRIDE", &fixture)
        .current_dir(&root)
        .output()
        .expect("execute lint script with override");

    let _ = std::fs::remove_file(&fixture);
    let _ = std::fs::remove_dir(&tmp_dir);

    assert!(
        !output.status.success(),
        "lint must exit non-zero on unmarked Utc::now() — stdout: {:?}, stderr: {:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("synthetic_unmarked_provider.rs"),
        "lint output must reference the offending file; got: {stdout}"
    );
}

#[test]
fn dos259_provider_clock_rng_lint_accepts_grandfathered_marker_in_synthetic_fixture() {
    // Companion regression: verify that a `dos259-grandfathered:` marker
    // within 3 lines above the Utc::now() call exempts it (not just by
    // virtue of the line numbers in the production glean_provider.rs).
    let tmp_dir = std::env::temp_dir().join(format!("dos259-lint-test-gf-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).expect("create temp dir");
    let fixture = tmp_dir.join("synthetic_grandfathered_provider.rs");
    let synthetic = "\
//! Synthetic provider-module fixture with a properly-marked grandfather call.
fn timestamp() -> String {
    // dos259-grandfathered: synthetic-test fixture; lint must accept this marker.
    chrono::Utc::now().to_rfc3339()
}
";
    std::fs::write(&fixture, synthetic).expect("write fixture");

    let root = workspace_root();
    let script = root.join("scripts/check_no_direct_clock_rng_in_provider_modules.sh");
    let output = Command::new("bash")
        .arg(&script)
        .env("DOS259_LINT_FILES_OVERRIDE", &fixture)
        .current_dir(&root)
        .output()
        .expect("execute lint script with override");

    let _ = std::fs::remove_file(&fixture);
    let _ = std::fs::remove_dir(&tmp_dir);

    assert!(
        output.status.success(),
        "lint must exit zero when the only call has a grandfathered marker — stderr: {:?}",
        String::from_utf8_lossy(&output.stderr),
    );
}
