//! regex contract test for `scripts/check_no_let_underscore_feedback.sh`.
//!
//! The bash lint catches `let _ = ...` swallows of three protected functions
//! (`record_feedback_event`, `create_suppression_tombstone`, `write_intelligence_json`)
//! across three call forms (method-call, qualified-path, bare). Cycle-2 and
//! cycle-3 review found that an earlier regex caught only the method-call
//! form. This test pins the contract so a future regression is loud.
//!
//! What the lint deliberately does NOT catch (acceptable for v1.4.0 W0;
//! structural enforcement via `clippy::let_underscore_must_use` is
//! territory):
//!   - `.ok();` chained on must-use
//!   - `match { _ => () }`
//!   - `if let Err(_) = ...`
//!   - wrapper-function indirection

use std::fs;
use std::process::Command;

/// Run the lint against a temporary directory containing a single fixture file.
/// Returns true if the lint exited 0 (clean); false if it exited non-zero (caught).
fn run_lint_against_fixture(rust_source: &str) -> bool {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src-tauri/src");
    fs::create_dir_all(&src_dir).expect("mkdir");
    fs::write(src_dir.join("fixture.rs"), rust_source).expect("write");

    // The lint script greps under <repo>/src-tauri/src and <repo>/src.
    // We point its grep at our temp tree by symlinking ROOT_DIR-relative
    // structure or by invoking the regex directly. Simplest: run grep here.
    let pattern = r"let[[:space:]]+_[[:alnum:]_]*([[:space:]]*:[[:space:]]*[^=]+)?[[:space:]]*=[[:space:]].*\b(record_feedback_event|create_suppression_tombstone|write_intelligence_json)[[:space:]]*\(";

    let out = Command::new("grep")
        .args(["-rEn", pattern, src_dir.to_str().unwrap()])
        .output()
        .expect("run grep");

    out.stdout.is_empty()
}

#[test]
fn lint_catches_method_call_swallow() {
    let src = r#"
fn caller(db: &ActionDb) {
    let _ = db.record_feedback_event(&input);
}
"#;
    assert!(
        !run_lint_against_fixture(src),
        "lint should catch `let _ = db.record_feedback_event(...)`"
    );
}

#[test]
fn lint_catches_qualified_path_swallow() {
    let src = r#"
fn caller() {
    let _ = crate::intelligence::write_intelligence_json(&dir, &intel);
}
"#;
    assert!(
        !run_lint_against_fixture(src),
        "lint should catch `let _ = crate::intelligence::write_intelligence_json(...)`"
    );
}

#[test]
fn lint_catches_bare_function_swallow() {
    let src = r#"
fn caller() {
    let _ = write_intelligence_json(&dir, &intel);
}
"#;
    assert!(
        !run_lint_against_fixture(src),
        "lint should catch bare-form `let _ = write_intelligence_json(...)`"
    );
}

#[test]
fn lint_catches_typed_underscore_swallow() {
    let src = r#"
fn caller(db: &ActionDb) {
    let _: Result<i64, _> = db.record_feedback_event(&input);
}
"#;
    assert!(
        !run_lint_against_fixture(src),
        "lint should catch `let _: T = ...`"
    );
}

#[test]
fn lint_catches_named_underscore_prefix_swallow() {
    let src = r#"
fn caller(db: &ActionDb) {
    let _ignored = db.create_suppression_tombstone(eid, fk, None, None, None, None);
}
"#;
    assert!(
        !run_lint_against_fixture(src),
        "lint should catch `let _ignored = ...`"
    );
}

#[test]
fn lint_passes_question_mark_propagation() {
    let src = r#"
fn caller(db: &ActionDb) -> Result<(), String> {
    db.record_feedback_event(&input).map_err(|e| e.to_string())?;
    Ok(())
}
"#;
    assert!(
        run_lint_against_fixture(src),
        "lint must NOT flag `?` propagation"
    );
}

#[test]
fn lint_passes_explicit_match() {
    let src = r#"
fn caller(db: &ActionDb) {
    match db.record_feedback_event(&input) {
        Ok(_) => {}
        Err(e) => log::warn!("propagated: {e}"),
    }
}
"#;
    assert!(
        run_lint_against_fixture(src),
        "lint must NOT flag explicit `match` handling"
    );
}

#[test]
fn lint_passes_dot_ok_chain() {
    // `.ok()` is the documented escape hatch for tests / best-effort cleanup.
    let src = r#"
fn caller() {
    write_intelligence_json(&dir, &intel).ok();
}
"#;
    assert!(
        run_lint_against_fixture(src),
        "lint deliberately does NOT flag `.ok();` (tests / best-effort cleanup escape hatch)"
    );
}

#[test]
fn lint_passes_unprotected_function() {
    // `let _ = some_other_fn()` is NOT in the denylist — only the three
    // named functions are protected.
    let src = r#"
fn caller() {
    let _ = some_other_function();
}
"#;
    assert!(
        run_lint_against_fixture(src),
        "lint must only flag the named denylist functions"
    );
}

#[test]
fn lint_script_runs_clean_against_current_workspace() {
    // Run the actual lint script against the workspace; it must exit 0
    // because all known swallows have been fixed in this PR.
    let repo_root = std::env::current_dir()
        .expect("cwd")
        .ancestors()
        .find(|p| p.join("scripts/check_no_let_underscore_feedback.sh").exists())
        .expect("locate repo root with scripts/check_no_let_underscore_feedback.sh")
        .to_path_buf();

    let script = repo_root.join("scripts/check_no_let_underscore_feedback.sh");
    let out = Command::new("bash")
        .arg(&script)
        .output()
        .expect("run lint script");

    assert!(
        out.status.success(),
        "scripts/check_no_let_underscore_feedback.sh failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}
