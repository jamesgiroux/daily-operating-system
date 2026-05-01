#[test]
fn ability_output_cannot_be_constructed_outside_provenance_module() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/external_ability_output_construction_fails.rs");
}
