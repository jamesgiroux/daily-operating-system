#[test]
fn pipeline_never_opens_paths_directly() {
    let source = std::fs::read_to_string("src/services/workspace_ingestion/pipeline.rs")
        .expect("pipeline.rs should be readable");

    let forbidden = [
        "File::open",
        "std::fs::File::open",
        "std::fs::File::options",
        "std::fs::OpenOptions",
        "tokio::fs::File::open",
        "tokio::fs::File::options",
        "tokio::fs::OpenOptions",
        "std::fs::read",
        "std::fs::read_to_string",
        "std::fs::metadata",
        "memmap",
        "std::process::Command",
    ];
    let forbidden_imports = [
        "OpenOptions",
        "tokio::fs",
        "read_to_string",
        "metadata",
        "memmap",
        "Command",
    ];

    for (line_no, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim_start();
        if line.starts_with("//") {
            continue;
        }
        if line.starts_with("use ") {
            for token in forbidden_imports {
                assert!(
                    !line.contains(token),
                    "pipeline.rs must not import path-opening helper {token:?} at line {}",
                    line_no + 1,
                );
            }
            continue;
        }
        for token in forbidden {
            assert!(
                !line.contains(token),
                "pipeline.rs must consume the validated File handle; forbidden token {token:?} at line {}",
                line_no + 1,
            );
        }
    }
}
