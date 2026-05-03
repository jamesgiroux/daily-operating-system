//! DOS-7 D4-2: lint regression tests for the claim-substrate guards.
//!
//! Each lint script in src-tauri/scripts/check_claim_*.sh enforces an
//! invariant from the DOS-7 plan §6 + amendment D. These tests run the
//! scripts against the live tree and assert clean exit, so a future
//! regression that violates the invariant fails CI loudly.

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

fn run_lint(script_rel_path: &str) -> (bool, String, String) {
    let root = repo_root();
    let script = root.join(script_rel_path);
    assert!(script.is_file(), "lint script missing: {}", script.display());
    let output = Command::new("bash")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run lint");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn lint_no_delete_on_claim_tables_passes_against_current_tree() {
    let (ok, stdout, stderr) = run_lint("src-tauri/scripts/check_intelligence_claims_no_delete.sh");
    assert!(
        ok,
        "no-DELETE lint failed:\nstdout: {}\nstderr: {}",
        stdout, stderr
    );
}

#[test]
fn lint_claim_writer_allowlist_passes_against_current_tree() {
    let (ok, stdout, stderr) = run_lint("src-tauri/scripts/check_claim_writer_allowlist.sh");
    assert!(
        ok,
        "claim-writer-allowlist lint failed:\nstdout: {}\nstderr: {}",
        stdout, stderr
    );
}

#[test]
fn lint_claim_immutability_passes_against_current_tree() {
    let (ok, stdout, stderr) =
        run_lint("src-tauri/scripts/check_claim_immutability_allowlist.sh");
    assert!(
        ok,
        "claim-immutability lint failed:\nstdout: {}\nstderr: {}",
        stdout, stderr
    );
}

/// L2 cycle-1 fix #5: every runtime legacy-dismissal-table write must
/// be paired with a `shadow_write_tombstone_claim` call within ±50
/// lines so the claim substrate stays in parity with legacy storage.
/// Without the pair, commit_claim PRE-GATE misses the dismissal and
/// the AI can re-surface the item on the next enrichment.
/// L2 cycle-19 regression: prove the immutability lint actually
/// catches a multi-column `SET dedup_key = ?, subject_ref = ?`
/// when the forbidden column is NOT the first SET target. Pre-fix,
/// the lint regex `SET[[:space:]]+(forbidden)` only matched
/// position-1 SET columns, so this shape passed silently.
///
/// The bad fixture is constructed dynamically (not as a source
/// literal) so that the lint scanning the LIVE tree doesn't trip
/// on this test's own source file.
#[test]
fn lint_immutability_catches_multi_column_subject_ref_set() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    // Build the fixture from constants so the test source never
    // contains the literal `subject_ref = ?` pattern that would
    // otherwise trip the live-tree lint scan when this test file
    // is itself within the lint roots.
    let forbidden = "subject".to_string() + "_" + "ref";
    let bad_sql = format!(
        "let _ = conn.execute(\n\
         \"UPDATE intelligence_claims \\\n\
          SET dedup_key = ?1, {forbidden} = ?2 \\\n\
          WHERE id = ?3\",\n\
         params,\n\
         );\n"
    );
    std::fs::write(tmp.path().join("src/bad_fixture.rs"), bad_sql)
        .expect("write bad fixture");

    let lint_path = repo_root().join("src-tauri/scripts/check_claim_immutability_allowlist.sh");
    let output = std::process::Command::new("bash")
        .arg(&lint_path)
        .current_dir(tmp.path())
        .output()
        .expect("run lint");
    assert!(
        !output.status.success(),
        "lint must FAIL on multi-column SET that includes a forbidden column \
         (cycle-19 regression). stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// L2 cycle-20 fix #1: prove the immutability lint catches
/// QUOTED forbidden identifiers in any SET position. SQLite
/// accepts `"col"`, `` `col` ``, and `[col]` — all should be
/// caught even when not the first SET target.
#[test]
fn lint_immutability_catches_quoted_subject_ref_in_multi_column_set() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    let forbidden = "subject".to_string() + "_" + "ref";
    // Single-line UPDATE so the quoted-identifier match is on the
    // same line as `UPDATE intelligence_claims`. The `\"…\"` in the
    // format string emits a literal `"col"` SQL-quoted form into
    // the fixture file.
    let bad_sql = format!(
        "let _ = conn.execute(\n\
         \"UPDATE intelligence_claims SET dedup_key = ?1, \"{forbidden}\" = ?2 WHERE id = ?3\",\n\
         params,\n\
         );\n"
    );
    std::fs::write(tmp.path().join("src/bad_quoted.rs"), bad_sql)
        .expect("write quoted bad fixture");

    let lint_path =
        repo_root().join("src-tauri/scripts/check_claim_immutability_allowlist.sh");
    let output = std::process::Command::new("bash")
        .arg(&lint_path)
        .current_dir(tmp.path())
        .output()
        .expect("run lint");
    assert!(
        !output.status.success(),
        "lint must FAIL on quoted forbidden identifier in non-leading SET position. \
         stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// L2 cycle-20 fix #2: prove the legacy dismissal pairing lint
/// catches `UPDATE briefing_callouts SET other = ?, dismissed_at = ?`
/// when `dismissed_at` is not the first SET target. Same blind
/// spot the immutability lint had pre-cycle-19.
#[test]
fn lint_legacy_dismissal_pairing_catches_non_leading_dismissed_at_set() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    let dismissed_field = "dismissed".to_string() + "_at";
    // briefing_callouts UPDATE with dismissed_at as the SECOND
    // SET target, no shadow_write_tombstone_claim nearby.
    let bad_sql = format!(
        "fn purge() {{\n\
         let _ = conn.execute(\n\
         \"UPDATE briefing_callouts SET updated_at = ?1, {dismissed_field} = ?2 WHERE id = ?3\",\n\
         params,\n\
         );\n\
         }}\n"
    );
    std::fs::write(tmp.path().join("src/bad_dismiss.rs"), bad_sql)
        .expect("write dismiss bad fixture");

    let lint_path =
        repo_root().join("src-tauri/scripts/check_legacy_dismissal_shadow_write_pairing.sh");
    let output = std::process::Command::new("bash")
        .arg(&lint_path)
        .current_dir(tmp.path())
        .output()
        .expect("run lint");
    assert!(
        !output.status.success(),
        "lint must FAIL on non-leading dismissed_at SET without shadow-write pairing. \
         stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// L2 cycle-21 regression: prove the legacy dismissal pairing lint
/// catches `UPDATE account_stakeholder_roles SET dismissed_at = ...`
/// without a shadow-write nearby. Cycle-1 audit missed this site
/// (remove_stakeholder_role_inner) AND the lint's UPDATE branch
/// only matched briefing_callouts, so the bug was double-shielded.
#[test]
fn lint_legacy_dismissal_pairing_catches_account_stakeholder_role_dismissal() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    let dismissed_field = "dismissed".to_string() + "_at";
    let bad_sql = format!(
        "fn evil() {{\n\
         let _ = conn.execute(\n\
         \"UPDATE account_stakeholder_roles SET {dismissed_field} = datetime('now') WHERE id = ?1\",\n\
         params,\n\
         );\n\
         }}\n"
    );
    std::fs::write(tmp.path().join("src/bad_role_dismiss.rs"), bad_sql)
        .expect("write fixture");

    let lint_path =
        repo_root().join("src-tauri/scripts/check_legacy_dismissal_shadow_write_pairing.sh");
    let output = std::process::Command::new("bash")
        .arg(&lint_path)
        .current_dir(tmp.path())
        .output()
        .expect("run lint");
    assert!(
        !output.status.success(),
        "lint must FAIL on UPDATE account_stakeholder_roles SET dismissed_at without shadow-write. \
         stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// L2 cycle-22 regression: prove the pairing lint catches a
/// MULTILINE `UPDATE account_stakeholder_roles ... SET ...
/// dismissed_at = ...` shape that splits across source lines via
/// Rust string-literal line continuation. Cycle-20's single-line
/// regex missed this; cycle-22 added a 7-line lookahead.
#[test]
fn lint_legacy_dismissal_pairing_catches_multiline_update_dismissed_at() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    let dismissed_field = "dismissed".to_string() + "_at";
    // Multi-line UPDATE: `UPDATE` on one line, `SET ... dismissed_at`
    // on a later line. No shadow-write.
    let bad_sql = format!(
        "fn evil() {{\n\
         let _ = conn.execute(\n\
         \"UPDATE account_stakeholder_roles \\\n\
          SET data_source = 'repair', {dismissed_field} = ?1 \\\n\
          WHERE account_id = ?2\",\n\
         params,\n\
         );\n\
         }}\n"
    );
    std::fs::write(tmp.path().join("src/bad_multiline.rs"), bad_sql)
        .expect("write fixture");

    let lint_path =
        repo_root().join("src-tauri/scripts/check_legacy_dismissal_shadow_write_pairing.sh");
    let output = std::process::Command::new("bash")
        .arg(&lint_path)
        .current_dir(tmp.path())
        .output()
        .expect("run lint");
    assert!(
        !output.status.success(),
        "lint must FAIL on multiline UPDATE ... dismissed_at without shadow-write. \
         stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// L2 cycle-22 regression: prove the pairing lint catches a
/// helper-call site (e.g. `db.upsert_linking_dismissal(...)`)
/// without a shadow-write nearby. Cycle-22 added direct
/// helper-name patterns since these calls bypass raw-SQL detection.
#[test]
fn lint_legacy_dismissal_pairing_catches_helper_call_without_shadow() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    let helper = "upsert".to_string() + "_" + "linking" + "_" + "dismissal";
    let bad_rs = format!(
        "fn evil(db: &ActionDb) {{\n\
         db.{helper}(owner_type, owner_id, entity_id, entity_type, None).unwrap();\n\
         }}\n"
    );
    std::fs::write(tmp.path().join("src/bad_helper.rs"), bad_rs)
        .expect("write fixture");

    let lint_path =
        repo_root().join("src-tauri/scripts/check_legacy_dismissal_shadow_write_pairing.sh");
    let output = std::process::Command::new("bash")
        .arg(&lint_path)
        .current_dir(tmp.path())
        .output()
        .expect("run lint");
    assert!(
        !output.status.success(),
        "lint must FAIL on db.upsert_linking_dismissal call without shadow-write. \
         stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn lint_legacy_dismissal_shadow_write_pairing_passes_against_current_tree() {
    let (ok, stdout, stderr) =
        run_lint("src-tauri/scripts/check_legacy_dismissal_shadow_write_pairing.sh");
    assert!(
        ok,
        "shadow-write pairing lint failed:\nstdout: {}\nstderr: {}",
        stdout, stderr
    );
}
