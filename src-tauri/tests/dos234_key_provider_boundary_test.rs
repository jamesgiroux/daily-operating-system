use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const SENSITIVE_KEY_TOKENS: &[&str] = &[
    "EncryptionKey",
    "as_hex(",
    "to_pragma(",
    "key_to_pragma",
    "get_or_create_key",
    "rotate_key",
    "rekey_database",
];

const ALLOWED_KEY_MATERIAL_PATHS: &[&str] = &[
    "src/db/",
    "src/db_service.rs",
    "src/db_backup.rs",
    "src/migrations.rs",
];

#[test]
fn dos234_key_material_stays_out_of_runtime_surfaces() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scan_roots = [
        manifest_dir.join("src"),
        manifest_dir.join("abilities-runtime/src"),
    ];

    let mut leaks = Vec::new();
    for root in scan_roots {
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let relative = path
                .strip_prefix(&manifest_dir)
                .expect("scan path under manifest dir");
            if is_allowed_key_material_path(relative) {
                continue;
            }

            let source = std::fs::read_to_string(path).expect("read source file");
            for token in SENSITIVE_KEY_TOKENS {
                if source.contains(token) {
                    leaks.push(format!("{} contains {token}", relative.display()));
                }
            }
        }
    }

    assert!(
        leaks.is_empty(),
        "DB key material/provider calls must stay out of Provenance, signal, telemetry, and abilities-runtime surfaces:\n{}",
        leaks.join("\n")
    );
}

fn is_allowed_key_material_path(relative: &Path) -> bool {
    let relative = relative.to_string_lossy();
    ALLOWED_KEY_MATERIAL_PATHS
        .iter()
        .any(|allowed| relative == *allowed || relative.starts_with(allowed))
}
