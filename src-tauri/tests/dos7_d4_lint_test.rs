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
