use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=../tests/dos209_mutation_catalog.txt");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let catalog_path = manifest_dir.join("../tests/dos209_mutation_catalog.txt");
    let catalog = fs::read_to_string(&catalog_path)?;

    let mut allowlist = BTreeSet::new();
    for line in catalog.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let symbol = line
            .split_once('|')
            .map(|(symbol, _)| symbol)
            .unwrap_or(line)
            .trim();

        let Some((path, _line_number)) = symbol.rsplit_once(':') else {
            continue;
        };

        allowlist.insert(format!("services::{path}"));
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let allowlist_path = out_dir.join("mutation_allowlist.rs");
    let mut generated = String::from("pub static MUTATION_ALLOWLIST: &[&str] = &[\n");
    for path in allowlist {
        generated.push_str("    ");
        generated.push_str(&format!("{path:?}"));
        generated.push_str(",\n");
    }
    generated.push_str("];\n");

    fs::write(allowlist_path, generated)
}
