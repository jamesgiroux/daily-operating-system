//! v1.4.5 W1-A — substrate-reinvention regression gate.
//!
//! Cycle 1–3 of W1-A's L0 review surfaced three substrate-reinvention class
//! findings (V1.1 invented `TrustFactorInput` colliding with canonical
//! `TrustFactorInputs`; V1.2 had local `SourceAttribution` colliding with the
//! canonical 7-field struct; V1.2 had `SourceType` mirroring canonical
//! `WorkspaceFileKind`). The structural fix is this CI grep gate: any
//! `pub struct/enum/trait` under `services/workspace_ingestion/` whose name
//! matches the canonical substrate primitive list fails the test.
//!
//! Gate scope: catches `pub struct`, `pub enum`, `pub trait`. Does NOT catch
//! `pub(crate)` definitions, `pub type` aliases, or same-shape renamed
//! clones — strengthening to a Rust-AST-aware lint is filed as Codebase
//! Maintenance follow-up. The current gate catches the actual reinvention
//! shape that fired three times during L0 (always `pub struct/enum/trait`),
//! which is the highest-value structural prevention available without a
//! lint rewrite.

use std::fs;
use std::path::PathBuf;

const REINVENTION_NAMES: &[&str] = &[
    "SourceAttribution",
    "SourceType",
    "TrustFactorInput",
    "SourceIdentifier",
    "ClaimProposal",
    "Provenance",
];

#[test]
fn no_substrate_reinvention_in_workspace_ingestion() {
    let dir: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("workspace_ingestion");

    let mut violations: Vec<String> = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}")) {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        for (lineno_zero, line) in source.lines().enumerate() {
            let lineno = lineno_zero + 1;
            for name in REINVENTION_NAMES {
                let needle_struct = format!("pub struct {name}");
                let needle_enum = format!("pub enum {name}");
                let needle_trait = format!("pub trait {name}");
                if line.contains(&needle_struct)
                    || line.contains(&needle_enum)
                    || line.contains(&needle_trait)
                {
                    violations.push(format!(
                        "{}:{}: reinvents canonical substrate primitive `{}` — \
                         consume the canonical type instead of defining a parallel",
                        path.display(),
                        lineno,
                        name
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "substrate-reinvention regression detected ({} violation(s)):\n  {}",
        violations.len(),
        violations.join("\n  ")
    );
}
