#[test]
fn temporal_trait_shape_compiles() {
    let t = trybuild::TestCases::new();
    t.pass("tests/trybuild/temporal/trait_shape.rs");
}
