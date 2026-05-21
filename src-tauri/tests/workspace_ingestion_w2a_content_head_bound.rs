use std::path::PathBuf;

#[test]
fn pipeline_uses_four_kib_content_head_constant_for_sniffing() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("src/services/workspace_ingestion/pipeline.rs"))
        .expect("read pipeline");
    assert!(source.contains("const CONTENT_HEAD_BYTES: usize = 4 * 1024;"));
    assert!(source.contains("CONTENT_HEAD_BYTES"));
    assert!(source.contains("auto_detect_category_pure(filename, content_head)"));
}
