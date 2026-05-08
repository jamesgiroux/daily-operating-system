pub fn write_behind_helper() {
    std::fs::write("target/ability-runtime-boundary-proof", b"forbidden").unwrap();
}
