#[test]
fn frontmatter_link_map_macro_shape_rejects_missing_required_keys() {
    std::env::set_var("DAILYOS_SRC_TAURI", env!("CARGO_MANIFEST_DIR"));
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/frontmatter_link_map_shape_missing_subject_type_fails.rs");
}
