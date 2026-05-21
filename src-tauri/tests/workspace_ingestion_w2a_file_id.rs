use dailyos_lib::services::workspace_ingestion::contracts::FileIdentity;
use dailyos_lib::services::workspace_ingestion::pipeline::file_id_from_identity;
use sha2::{Digest, Sha256};

#[test]
fn file_id_uses_sha256_of_workspace_relative_path() {
    let root = std::path::PathBuf::from("/tmp/workspace");
    let identity = FileIdentity {
        canonical_path: root.join("Accounts/acme/file.md"),
        device: 1,
        inode: 2,
    };
    let file_id = file_id_from_identity(&identity, &root).expect("file id");
    let digest = Sha256::digest(b"Accounts/acme/file.md");
    assert_eq!(file_id, hex::encode(digest)[..16].to_string());
}
