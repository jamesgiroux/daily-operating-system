use std::path::PathBuf;

#[test]
fn entity_intake_uses_self_crate_imports_inside_abilities_runtime() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let producer = std::fs::read_to_string(
        root.join("abilities-runtime/src/abilities/entity_intake/producer.rs"),
    )
    .expect("read producer");
    let contracts = std::fs::read_to_string(
        root.join("abilities-runtime/src/abilities/entity_intake/contracts.rs"),
    )
    .expect("read contracts");

    assert!(
        producer.contains("use crate::") && contracts.contains("use crate::abilities::trust::types::TrustBand;"),
        "entity_intake must use self-crate imports"
    );
    assert!(
        !producer.contains("abilities_runtime::") && !contracts.contains("abilities_runtime::"),
        "abilities-runtime modules must not import themselves through abilities_runtime::"
    );
}
