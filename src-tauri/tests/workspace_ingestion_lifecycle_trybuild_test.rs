#[test]
fn workspace_file_lifecycle_requires_core_provenance_fields() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/workspace_file_lifecycle_missing_core_fields_fails.rs");
}
