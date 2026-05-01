#[test]
fn trybuild_runner() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/*_fails.rs");
    t.pass("tests/trybuild/*_passes.rs");
}
