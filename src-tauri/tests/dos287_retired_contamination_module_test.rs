use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn dos287_retired_contamination_module_is_absent() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    assert!(
        !manifest_dir
            .join("src/intelligence/contamination.rs")
            .exists(),
        "retired contamination module file must stay deleted"
    );

    let module_root = fs::read_to_string(manifest_dir.join("src/intelligence/mod.rs"))
        .expect("read intelligence module root");
    assert!(
        !module_root.contains(concat!("pub mod ", "contamination;")),
        "retired contamination module must not be re-exported"
    );

    let forbidden = [
        concat!("intelligence::", "contamination"),
        concat!("process_", "contamination"),
        concat!("DAILYOS_", "CONTAMINATION_VALIDATION"),
        concat!("devtools_audit_cross_", "contamination"),
        concat!("devtools_clear_", "contaminated_enrichment"),
        concat!("SkipDueTo", "Contamination"),
    ];

    for path in rust_sources_under(&manifest_dir.join("src")) {
        let source = fs::read_to_string(&path).unwrap_or_default();
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "{} still contains retired symbol `{needle}`",
                path.display()
            );
        }
    }
}

fn rust_sources_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_rust_sources(root, &mut out);
    out
}

fn collect_rust_sources(path: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
