use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[test]
fn clippy_denies_let_underscore_must_use() {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wrapper_dir = manifest_dir
        .parent()
        .expect("abilities-runtime manifest has workspace parent")
        .join("target/trybuild-clippy");
    let wrapper = wrapper_dir.join("cargo-clippy-wrapper.sh");

    fs::create_dir_all(&wrapper_dir).expect("create trybuild clippy wrapper directory");
    fs::write(
        &wrapper,
        r#"#!/usr/bin/env bash
set -euo pipefail

args=("$@")
for i in "${!args[@]}"; do
  if [[ "${args[$i]}" == "check" ]]; then
    args[$i]="clippy"
    if [[ -f Cargo.toml ]] && ! grep -q '^let_underscore_must_use = "deny"$' Cargo.toml; then
      cat >> Cargo.toml <<'TOML'

[workspace.lints.clippy]
let_underscore_must_use = "deny"

[lints]
workspace = true
TOML
    fi
    exec "${REAL_CARGO:?}" "${args[@]}"
  fi
done

exec "${REAL_CARGO:?}" "$@"
"#,
    )
    .expect("write trybuild clippy wrapper");

    let mut permissions = fs::metadata(&wrapper)
        .expect("stat trybuild clippy wrapper")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&wrapper, permissions).expect("mark trybuild clippy wrapper executable");

    env::set_var("REAL_CARGO", cargo);
    env::set_var("CARGO", &wrapper);

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/lints/*.rs");
}
