use std::path::PathBuf;

#[test]
fn pipeline_rewinds_file_before_extractor_handoff() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("src/services/workspace_ingestion/pipeline.rs"))
        .expect("read pipeline");
    let seek = source
        .find(".seek(SeekFrom::Start(0))")
        .expect("seek before extractor");
    let extract = source.find(".extract(").expect("extract call");
    assert!(
        seek < extract,
        "pipeline must rewind before Extractor::extract"
    );
}
