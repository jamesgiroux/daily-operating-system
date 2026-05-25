//! v1.4.5 W1-A — substrate-reinvention regression gate.
//!
//! Cycle 1–3 of W1-A's L0 review surfaced three substrate-reinvention class
//! findings (V1.1 invented `TrustFactorInput` colliding with canonical
//! `TrustFactorInputs`; V1.2 had local `SourceAttribution` colliding with the
//! canonical 7-field struct; V1.2 had `SourceType` mirroring canonical
//! `WorkspaceFileKind`). The structural fix is this AST-aware CI gate: any
//! exported struct/enum/trait/type definitions under `services/workspace_ingestion/`
//! whose name matches the canonical substrate primitive list fail the test.
//!
//! Gate scope: parses Rust items instead of grepping so it catches `pub`,
//! `pub(crate)`, `pub(super)`, `pub(in ...)`, and `pub type` alias shapes.
//! Same-shape renamed clones still need human review; this gate prevents the
//! named reinvention class that fired during L0 from drifting back in.

use std::fs;
use std::path::PathBuf;

use syn::{Item, Type, Visibility};

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
        let parsed = syn::parse_file(&source).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
        for item in &parsed.items {
            inspect_item(&path, item, &mut violations);
        }
    }

    assert!(
        violations.is_empty(),
        "substrate-reinvention regression detected ({} violation(s)):\n  {}",
        violations.len(),
        violations.join("\n  ")
    );
}

fn inspect_item(path: &std::path::Path, item: &Item, violations: &mut Vec<String>) {
    match item {
        Item::Struct(item) if is_exported(&item.vis) => {
            push_if_reinvention(path, "struct", &item.ident.to_string(), violations);
        }
        Item::Enum(item) if is_exported(&item.vis) => {
            push_if_reinvention(path, "enum", &item.ident.to_string(), violations);
        }
        Item::Trait(item) if is_exported(&item.vis) => {
            push_if_reinvention(path, "trait", &item.ident.to_string(), violations);
        }
        Item::Type(item) if is_exported(&item.vis) => {
            let alias_name = item.ident.to_string();
            push_if_reinvention(path, "type alias", &alias_name, violations);
            if let Some(target_name) = type_leaf_ident(&item.ty) {
                if is_reinvention_name(&target_name) && !is_reinvention_name(&alias_name) {
                    violations.push(format!(
                        "{}: exported type alias `{alias_name}` points at canonical substrate \
                         primitive `{target_name}` — re-export the canonical type instead",
                        path.display()
                    ));
                }
            }
        }
        Item::Mod(item) => {
            if let Some((_, items)) = &item.content {
                for nested in items {
                    inspect_item(path, nested, violations);
                }
            }
        }
        _ => {}
    }
}

fn is_exported(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_) | Visibility::Restricted(_))
}

fn push_if_reinvention(
    path: &std::path::Path,
    kind: &str,
    name: &str,
    violations: &mut Vec<String>,
) {
    if is_reinvention_name(name) {
        violations.push(format!(
            "{}: exported {kind} `{name}` reinvents a canonical substrate primitive — \
             consume the canonical type instead of defining a parallel",
            path.display()
        ));
    }
}

fn is_reinvention_name(name: &str) -> bool {
    REINVENTION_NAMES.contains(&name)
}

fn type_leaf_ident(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        _ => None,
    }
}
