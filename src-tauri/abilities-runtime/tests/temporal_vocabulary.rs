use std::fs;
use std::path::Path;

#[test]
fn temporal_runtime_module_stays_domain_neutral() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("abilities")
        .join("temporal");
    let mut violations = Vec::new();
    collect_vocabulary_violations(&root, &mut violations);

    assert!(
        violations.is_empty(),
        "domain-specific vocabulary leaked into abilities/temporal:\n{}",
        violations.join("\n")
    );
}

fn collect_vocabulary_violations(path: &Path, violations: &mut Vec<String>) {
    let entries = fs::read_dir(path).expect("read temporal module dir");
    for entry in entries {
        let entry = entry.expect("read temporal module entry");
        let path = entry.path();
        if path.is_dir() {
            collect_vocabulary_violations(&path, violations);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let source = fs::read_to_string(&path).expect("read temporal source");
        let lowered = source.to_ascii_lowercase();
        for term in ["account", "churn", "expansion"] {
            if lowered.contains(term) {
                violations.push(format!("{} contains `{term}`", path.display()));
            }
        }
    }
}
