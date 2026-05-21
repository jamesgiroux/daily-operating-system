//! AC-340.3 / AC-340.5: receipt boundary snapshot harness.
//!
//! Loads every JSON fixture under `tests/claim_receipt_boundary/` and
//! asserts that every key is in
//! `services::claim_receipt::boundary::RECEIPT_ALLOWED_FIELDS`. Fixtures
//! cover the (`SurfaceContext` × `ClaimSensitivity`) cell matrix. A new
//! cell or a renamed field forces an intentional contract update.

use std::fs;
use std::path::PathBuf;

use dailyos_lib::services::claim_receipt::boundary::{
    AUDIT_ONLY_DENYLIST, RECEIPT_ALLOWED_FIELDS,
};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/claim_receipt_boundary")
}

fn fixture_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(fixtures_dir())
        .expect("fixtures dir exists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    files.sort();
    files
}

#[test]
fn fixture_matrix_is_complete() {
    let expected_cells = [
        "actions_work",
        "entity_detail",
        "daily_briefing",
        "meeting_detail",
        "mcp",
    ];
    let expected_sensitivities = ["public", "internal", "confidential", "user_only"];

    let names: Vec<String> = fixture_files()
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .collect();

    for surface in &expected_cells {
        for sensitivity in &expected_sensitivities {
            let expected = format!("{surface}__{sensitivity}");
            assert!(
                names.contains(&expected),
                "missing fixture for ({surface}, {sensitivity}); update L0 §5.8 \
                 snapshot matrix"
            );
        }
    }
}

#[test]
fn every_fixture_key_is_allowlisted() {
    let mut violations: Vec<String> = Vec::new();

    for path in fixture_files() {
        let body = fs::read_to_string(&path).expect("read fixture");
        let value: serde_json::Value =
            serde_json::from_str(&body).expect("fixture must be valid JSON");
        let obj = value
            .as_object()
            .unwrap_or_else(|| panic!("fixture {path:?} must be a JSON object"));

        for key in obj.keys() {
            let allowed = RECEIPT_ALLOWED_FIELDS.iter().any(|n| n == key);
            if !allowed {
                violations.push(format!("{}: disallowed field {:?}", path.display(), key));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "DOS-340 boundary leak in receipt fixtures:\n{}",
        violations.join("\n")
    );
}

#[test]
fn no_fixture_contains_denylisted_field() {
    let mut violations: Vec<String> = Vec::new();

    for path in fixture_files() {
        let body = fs::read_to_string(&path).expect("read fixture");
        let value: serde_json::Value =
            serde_json::from_str(&body).expect("fixture must be valid JSON");
        let obj = value
            .as_object()
            .unwrap_or_else(|| panic!("fixture {path:?} must be a JSON object"));

        for key in obj.keys() {
            if AUDIT_ONLY_DENYLIST.iter().any(|n| n == key) {
                violations.push(format!(
                    "{}: AUDIT_ONLY_DENYLIST field {:?} leaked to receipt",
                    path.display(),
                    key
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "DOS-340 audit-only leak in receipt fixtures:\n{}",
        violations.join("\n")
    );
}

#[test]
fn fixture_files_are_nonempty() {
    let count = fixture_files().len();
    // 5 surfaces × 4 sensitivities = 20.
    assert_eq!(
        count, 20,
        "expected 20 fixtures (5 surfaces × 4 sensitivities), got {count}"
    );
}
