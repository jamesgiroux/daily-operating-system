use dailyos_lib::services::workspace_ingestion::pipeline::IngestError;

#[test]
fn io_error_display_redacts_paths_to_kind_and_os_code() {
    let error = IngestError::Io(std::io::Error::from_raw_os_error(2));
    let rendered = error.to_string();
    assert!(rendered.contains("kind="));
    assert!(rendered.contains("os="));
    assert!(!rendered.contains("/tmp/"));
}
