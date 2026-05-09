fn fallible_side_effect() -> Result<(), &'static str> {
    Ok(())
}

fn main() {
    let _ = fallible_side_effect();
}
